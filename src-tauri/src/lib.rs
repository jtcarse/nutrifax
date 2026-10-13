use base64::{
    Engine as _,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Manager};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{Duration, timeout},
};
use url::Url;

const AUTH: &str = "https://auth.openai.com";
const API: &str = "https://api.openai.com/v1";
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

#[derive(Default)]
struct AppState {
    session: Mutex<Option<Session>>,
    status: Mutex<SignInStatus>,
}
struct Session {
    token: String,
}
#[derive(Clone, Default, Serialize)]
struct SignInStatus {
    state: String,
    message: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct HostRecord {
    host_id: String,
    client_id: Option<String>,
    subject: Option<String>,
}
#[derive(Deserialize)]
struct TokenReply {
    access_token: String,
    id_token: String,
    scope: String,
}
#[derive(Deserialize)]
struct Claims {
    sub: String,
    nonce: String,
}
#[derive(Deserialize)]
struct ModelCatalog {
    models: Vec<Model>,
}
#[derive(Deserialize)]
struct Model {
    slug: String,
    visibility: Option<String>,
}

fn random_urlsafe() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
fn host_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|_| "Cannot find app configuration folder".to_string())?;
    fs::create_dir_all(&dir).map_err(|_| "Cannot create app configuration folder".to_string())?;
    Ok(dir.join("chatgpt-host.json"))
}
fn save_host(app: &AppHandle, record: &HostRecord) -> Result<(), String> {
    let path = host_path(app)?;
    let temp = path.with_extension("tmp");
    fs::write(
        &temp,
        serde_json::to_vec(record).map_err(|_| "Cannot serialize host record")?,
    )
    .map_err(|_| "Cannot save host record")?;
    fs::rename(temp, path).map_err(|_| "Cannot finish saving host record".into())
}
fn host_record(app: &AppHandle) -> Result<HostRecord, String> {
    let path = host_path(app)?;
    if path.exists() {
        return serde_json::from_slice(
            &fs::read(path).map_err(|_| "Cannot read ChatGPT host record")?,
        )
        .map_err(|_| "Invalid ChatGPT host record".into());
    }
    let record = HostRecord {
        host_id: format!("urn:uuid:{}", uuid::Uuid::new_v4()),
        client_id: None,
        subject: None,
    };
    save_host(app, &record)?;
    Ok(record)
}

#[tauri::command]
async fn begin_sign_in(
    app: AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| "Cannot start loopback callback on this device".to_string())?;
    let port = listener
        .local_addr()
        .map_err(|_| "Cannot determine callback port")?
        .port();
    let redirect = format!("http://127.0.0.1:{port}/auth/callback");
    let host = host_record(&app)?;
    let verifier = random_urlsafe();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let csrf = random_urlsafe();
    let nonce = random_urlsafe();
    let mut url = Url::parse(&format!("{AUTH}/api/accounts/authorize"))
        .map_err(|_| "Invalid authorization URL")?;
    url.query_pairs_mut()
        .append_pair(
            "client_id",
            host.client_id.as_deref().unwrap_or("dynamic_agent_client"),
        )
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect)
        .append_pair("scope", SCOPE)
        .append_pair("resource", API)
        .append_pair("state", &csrf)
        .append_pair("nonce", &nonce)
        .append_pair("code_challenge_method", "S256")
        .append_pair("code_challenge", &challenge)
        .append_pair("ext_agent_host_id", &host.host_id)
        .append_pair("agent_name_hint", "Nutrifax");
    *state
        .status
        .lock()
        .map_err(|_| "Sign-in state unavailable")? = SignInStatus {
        state: "pending".into(),
        message: None,
    };
    let shared = state.inner().clone();
    tauri::async_runtime::spawn(async move {
        let outcome = timeout(
            Duration::from_secs(120),
            finish_sign_in(listener, redirect, verifier, csrf, nonce, host, &app),
        )
        .await;
        match outcome {
            Ok(Ok(session)) => {
                if let Ok(mut lock) = shared.session.lock() {
                    *lock = Some(session);
                }
                if let Ok(mut lock) = shared.status.lock() {
                    *lock = SignInStatus {
                        state: "ready".into(),
                        message: None,
                    };
                }
            }
            other => {
                let detail = match other {
                    Ok(Err(error)) => error,
                    Err(_) => "Browser callback timed out; retry sign-in".into(),
                    _ => unreachable!(),
                };
                if let Ok(mut lock) = shared.status.lock() {
                    *lock = SignInStatus {
                        state: "error".into(),
                        message: Some(detail),
                    };
                }
            }
        }
    });
    Ok(url.to_string())
}

async fn finish_sign_in(
    listener: TcpListener,
    redirect: String,
    verifier: String,
    csrf: String,
    nonce: String,
    mut host: HostRecord,
    app: &AppHandle,
) -> Result<Session, String> {
    let (mut socket, peer) = listener
        .accept()
        .await
        .map_err(|_| "Browser callback failed")?;
    if !peer.ip().is_loopback() {
        return Err("Callback did not come from this device".into());
    }
    let mut buf = [0u8; 8192];
    let read = socket
        .read(&mut buf)
        .await
        .map_err(|_| "Cannot read browser callback")?;
    let request = std::str::from_utf8(&buf[..read]).map_err(|_| "Invalid callback request")?;
    let target = request
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|line| line.split(' ').next())
        .ok_or("Invalid browser callback")?;
    let callback =
        Url::parse(&format!("http://127.0.0.1{target}")).map_err(|_| "Invalid callback URL")?;
    if callback.path() != "/auth/callback" {
        return Err("Unexpected callback path".into());
    }
    let params: std::collections::HashMap<_, _> = callback
        .query_pairs()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let page = "<p>Return to Nutrifax to see the sign-in result. You may close this tab.</p>";
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    let _ = socket.write_all(reply.as_bytes()).await;
    if params.get("state") != Some(&csrf) {
        return Err("Sign-in state mismatch; retry sign-in".into());
    }
    if let Some(error) = params.get("error") {
        return Err(format!(
            "Authorization declined or failed ({error}); retry sign-in"
        ));
    }
    let code = params
        .get("code")
        .ok_or("Authorization code missing; retry sign-in")?;
    let client_id = match (&host.client_id, params.get("client_id")) {
        (Some(saved), Some(returned)) if saved != returned => {
            return Err("Callback client ID mismatch; retry sign-in".into());
        }
        (Some(saved), _) => saved.clone(),
        (None, Some(issued)) if issued.starts_with("oaiapp_") => issued.clone(),
        _ => return Err("ChatGPT registration did not issue a client ID".into()),
    };
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{AUTH}/api/accounts/oauth/token"))
        .form(&[
            ("grant_type", "authorization_code"),
            ("client_id", client_id.as_str()),
            ("code", code.as_str()),
            ("code_verifier", verifier.as_str()),
            ("redirect_uri", redirect.as_str()),
            ("resource", API),
        ])
        .send()
        .await
        .map_err(|_| "Could not contact OpenAI token service; retry sign-in")?;
    if !response.status().is_success() {
        return Err(format!(
            "OpenAI rejected the authorization code (HTTP {}); retry sign-in",
            response.status()
        ));
    }
    let token: TokenReply = response
        .json()
        .await
        .map_err(|_| "OpenAI returned an invalid token response")?;
    let claims = verify_id_token(&client, &token.id_token, &client_id, &nonce).await?;
    if host.subject.as_ref().is_some_and(|old| old != &claims.sub) {
        return Err("A different ChatGPT account was selected; clear the local host record to switch accounts".into());
    }
    if !token
        .scope
        .split_whitespace()
        .any(|scope| scope == "chatgpt.tokens.use.direct")
    {
        return Err(
            "ChatGPT plan usage was not granted; enable it in the authorization screen and retry"
                .into(),
        );
    }
    host.client_id = Some(client_id);
    host.subject = Some(claims.sub);
    save_host(app, &host)?;
    Ok(Session {
        token: token.access_token,
    })
}

async fn verify_id_token(
    client: &reqwest::Client,
    id_token: &str,
    client_id: &str,
    nonce: &str,
) -> Result<Claims, String> {
    let header = decode_header(id_token).map_err(|_| "Invalid OpenAI ID token")?;
    let kid = header.kid.ok_or("OpenAI ID token has no key ID")?;
    let jwks: JwkSet = client
        .get(format!("{AUTH}/.well-known/jwks.json"))
        .send()
        .await
        .map_err(|_| "Cannot fetch OpenAI signing keys")?
        .error_for_status()
        .map_err(|_| "OpenAI signing keys unavailable")?
        .json()
        .await
        .map_err(|_| "Invalid OpenAI signing keys")?;
    let jwk = jwks.find(&kid).ok_or("OpenAI signing key not found")?;
    let key = DecodingKey::from_jwk(jwk).map_err(|_| "Unsupported OpenAI signing key")?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[AUTH]);
    validation.set_audience(&[client_id]);
    let claims = decode::<Claims>(id_token, &key, &validation)
        .map_err(|_| "OpenAI ID token validation failed")?
        .claims;
    if claims.nonce != nonce {
        return Err("OpenAI ID token nonce mismatch".into());
    }
    Ok(claims)
}

#[tauri::command]
fn sign_in_status(state: tauri::State<'_, Arc<AppState>>) -> Result<SignInStatus, String> {
    state
        .status
        .lock()
        .map(|v| v.clone())
        .map_err(|_| "Sign-in state unavailable".into())
}

#[tauri::command]
async fn analyze_pdf(
    name: String,
    data: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let token = state
        .session
        .lock()
        .map_err(|_| "ChatGPT connection unavailable")?
        .as_ref()
        .ok_or("Sign in with ChatGPT before analysis")?
        .token
        .clone();
    if data.len() > 14 * 1024 * 1024 {
        return Err("PDF exceeds the 10 MB feasibility limit".into());
    }
    let bytes = STANDARD
        .decode(&data)
        .map_err(|_| "Invalid PDF data from file picker")?;
    if bytes.len() < 5 || &bytes[..5] != b"%PDF-" {
        return Err("Choose a valid PDF and retry".into());
    }
    if bytes.len() > 10 * 1024 * 1024 {
        return Err("PDF exceeds the 10 MB feasibility limit".into());
    }
    let client = reqwest::Client::new();
    let model_reply = client
        .get(format!("{API}/models"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|_| "Cannot reach OpenAI model catalog")?;
    if !model_reply.status().is_success() {
        return Err(provider_error(model_reply).await);
    }
    let catalog: ModelCatalog = model_reply
        .json()
        .await
        .map_err(|_| "Invalid OpenAI model catalog")?;
    let model = catalog
        .models
        .iter()
        .find(|m| m.visibility.as_deref() == Some("list") && m.slug.starts_with("gpt-"))
        .ok_or("No listed GPT model is available for this ChatGPT account")?;
    let body = serde_json::json!({ "model": model.slug, "input": [{ "role": "user", "content": [
        { "type": "input_file", "filename": name, "file_data": format!("data:application/pdf;base64,{data}") },
        { "type": "input_text", "text": "Read this text-based restaurant nutrition PDF. Briefly identify its restaurant, menu item names, serving text, and nutrition values that are explicitly present. Preserve uncertainty such as <1, >1, and N/A as written. Do not invent missing values. Cite page numbers when possible." }
    ] }], "store": false, "stream": true });
    let response = client
        .post(format!("{API}/responses"))
        .bearer_auth(&token)
        .json(&body)
        .send()
        .await
        .map_err(|_| "Cannot reach OpenAI inference service")?;
    if !response.status().is_success() {
        return Err(provider_error(response).await);
    }
    let stream = response
        .text()
        .await
        .map_err(|_| "OpenAI inference stream interrupted")?;
    parse_response_stream(&stream)
}

async fn provider_error(response: reqwest::Response) -> String {
    let status = response.status();
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let value: serde_json::Value = response.json().await.unwrap_or_default();
    let code = value
        .pointer("/error/code")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let detail = match code {
        "subscription_sharing_user_not_eligible" => "This ChatGPT account or workspace is not eligible for plan usage; choose an eligible account".into(),
        "subscription_sharing_usage_limit_exceeded" => "ChatGPT app usage limit reached; review ChatGPT Settings → Usage and retry later".into(),
        "subscription_sharing_unsupported_capability" => "OpenAI does not support this PDF/model request on this connection; record this blocker before changing the integration".into(),
        _ if status.as_u16() == 401 => "ChatGPT permission was rejected; sign in again and grant plan usage".into(),
        _ if status.as_u16() == 403 => format!("OpenAI denied this request (HTTP 403, code {code}); check account eligibility and region"),
        _ => format!("OpenAI request failed (HTTP {status}, code {code}); retry or record the failure"),
    };
    if request_id.is_empty() {
        detail
    } else {
        format!("{detail} (request ID {request_id})")
    }
}

fn parse_response_stream(stream: &str) -> Result<String, String> {
    let mut text = String::new();
    let mut completed = false;
    for line in stream.lines() {
        let Some(data) = line.strip_prefix("data: ") else {
            continue;
        };
        if data == "[DONE]" {
            continue;
        }
        let value: serde_json::Value =
            serde_json::from_str(data).map_err(|_| "Malformed OpenAI inference event")?;
        match value.get("type").and_then(|v| v.as_str()) {
            Some("response.output_text.delta") => {
                if let Some(delta) = value.get("delta").and_then(|v| v.as_str()) {
                    text.push_str(delta);
                }
            }
            Some("response.completed") => completed = true,
            Some("response.failed") => {
                return Err(format!(
                    "OpenAI inference failed: {}",
                    value
                        .pointer("/response/error/code")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error")
                ));
            }
            Some("response.incomplete") => {
                return Err("OpenAI inference ended incomplete; retry with a smaller PDF".into());
            }
            _ => {}
        }
    }
    if !completed {
        return Err("OpenAI stream ended before response.completed".into());
    }
    if text.trim().is_empty() {
        return Err("OpenAI completed with an empty result".into());
    }
    Ok(text)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(Arc::<AppState>::default())
        .invoke_handler(tauri::generate_handler![
            begin_sign_in,
            sign_in_status,
            analyze_pdf
        ])
        .run(tauri::generate_context!())
        .expect("error while running Nutrifax");
}

#[cfg(test)]
mod tests {
    use super::parse_response_stream;
    #[test]
    fn completed_result_requires_text() {
        assert_eq!(parse_response_stream("data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}\n\ndata: {\"type\":\"response.completed\"}\n\n").unwrap(), "Hello");
        assert!(parse_response_stream("data: {\"type\":\"response.completed\"}\n\n").is_err());
        assert!(parse_response_stream("data: {\"type\":\"response.failed\"}\n\n").is_err());
    }
}
