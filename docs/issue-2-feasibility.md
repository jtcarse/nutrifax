# Issue #2: Windows and Android ChatGPT PDF feasibility

Checked 2026-10-12. This is a feasibility build for [issue #2](https://github.com/jtcarse/nutrifax/issues/2), under [PRD #1](https://github.com/jtcarse/nutrifax/issues/1). It does not implement menu-item review, labels, URL acquisition, or a saved library.

## Current documented mechanism

- [Sign in with ChatGPT overview](https://developers.openai.com/siwc/token-sharing-open-source) describes open-source app registration, an eligible ChatGPT plan grant, and OAuth access tokens for Responses API requests. Identity sign-in alone does not grant plan usage.
- [Registration and sign-in](https://developers.openai.com/siwc/token-sharing-open-source/sign-in) specifies dynamic client registration, `127.0.0.1` HTTP callback, PKCE, nonce and state, token exchange, ID token validation, and checking the granted `chatgpt.tokens.use.direct` scope. The same-device callback must still be verified on Android.
- [Models and inference](https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference) specifies account model listing and `POST https://api.openai.com/v1/responses` with the OAuth bearer token, `store: false`, `stream: true`, and a completed stream event. A listed model is not proof that PDF inference works.
- [Preview limitations](https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations) says text, images, and file inputs are supported when the selected model accepts them; the Files upload API is excluded. [File inputs](https://developers.openai.com/api/docs/guides/file-inputs) documents inline base64 PDF `input_file` data on the Responses API. The app sends that inline data only from Analyze PDF.
- [Errors and recovery](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery) distinguishes denied consent, account eligibility, usage limits, unsupported capability, and provider failures. The app surfaces those branches rather than treating them as an empty result.
- [Tauri dialog](https://v2.tauri.app/plugin/dialog/) documents native file selection on Windows and Android, with Android returning a content URI; the Tauri filesystem plugin reads that URI.

These documents establish a supported integration path, not entitlement for this account or live success on either device. Nutrifax is currently an open-source local app. A future paid or remotely hosted distribution would require revisiting the documented eligibility path.

## Implemented behavior and limits

The native picker reads a local PDF of at most 10 MB. Selection retains bytes in the app and makes no OpenAI call. Continue with ChatGPT launches the system browser and a loopback callback listener in the same app process. The app verifies OAuth state, PKCE exchange, the ID-token signature, issuer, audience, expiration, nonce, and plan-use scope. It keeps the bearer token only in memory for this session; a stable host identifier and issued client ID are stored locally, with no credentials committed or logged. Analyze PDF has the explicit upload notice and sends inline PDF data through the documented Responses route. It displays completed inference text or an actionable error. The selected account's first listed GPT model is used; PDF support for that specific model still requires a live request.

The 10 MB cap and session-only token handling are feasibility limits. After expiration, sign in again. The app does not claim background token renewal or multi-account management. Its output is exploratory text, not validated nutrition facts for label export.

## Build and controlled checks

Environment observed: Windows, Node 24.15.0, npm 11.14.1, Rust 1.95.0, Tauri CLI 2.12.2, dialog plugin 2.8.2, filesystem plugin 2.7.0. Android SDK and NDK 23.2.8568313 were present. No Android device appeared in `adb devices`.

| Command | Result |
| --- | --- |
| `npm run build` | Passed TypeScript and Vite build. |
| `npm test` | Passed 8 controlled workflow tests, including no analysis on selection, notice, successful result, and file/sign-in/provider/empty-result failures. |
| `cargo check --manifest-path src-tauri/Cargo.toml` | Passed Windows Rust check. |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Passed stream completion test. |
| `./node_modules/.bin/tauri.cmd build --debug --no-bundle` | Passed Windows native binary build. |
| `npm run tauri -- android init -- --ci` | Generated Android project. Use the npm script form; direct invocation generated a broken Gradle CLI command on this machine. |
| `npm run tauri -- android build -- --debug --apk --target aarch64` | Passed Android arm64 debug APK build after npm-script initialization. |

The Windows binary is `src-tauri/target/debug/nutrifax.exe`. The Android debug APK is `src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`. These build outputs are ignored and were rebuilt after the last source edit.

## Human-assisted live checks: pending

Use an eligible ChatGPT account. Do not send credentials, authorization URLs, tokens, PDF bytes, or private restaurant PDFs to the issue, PR, logs, or agent. Use a small local text-based PDF that the owner is authorized to send to OpenAI. Record only device/OS version, app commit, selected PDF description, observed result or sanitized error code/request ID, and time.

### Windows

1. Run `npm ci`, then `npm run tauri -- build -- --debug --no-bundle`, and launch `src-tauri/target/debug/nutrifax.exe`.
2. Choose a local text-based PDF. Verify its name appears and no OpenAI request occurs yet. Cancel and invalid-file cases should show appropriate behavior.
3. Select Continue with ChatGPT. Finish authorization in the system browser and grant plan usage. Return to the app and confirm it reports plan permission. A completed sign-in is not an inference pass.
4. Read the notice, press Analyze PDF, and verify that the result contains information actually present in the PDF. Inspect inequalities and missing values without assuming invented values are correct. Record an actionable error if the account/model/provider denies or fails the request.

### Real Android device

1. With Android SDK, NDK, Rust Android target, and an arm64 device connected via USB debugging, run `npm ci`, `npm run tauri -- android init -- --ci`, then `npm run tauri -- android build -- --debug --apk --target aarch64`.
2. Confirm `adb devices` lists the real device. Install the APK with `adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk`.
3. On that phone, choose a local PDF with the native picker and verify the name appears before any AI request. Launch ChatGPT sign-in from the app and complete it in the phone's system browser. Confirm the browser reaches the app's `127.0.0.1` callback on the **same phone**, then return to Nutrifax and confirm plan permission. Do not infer callback success from emulator behavior or from a desktop browser.
4. Read the notice, press Analyze PDF, and verify a real completed PDF inference result on the phone. Record a sanitized error and where the flow stopped if it fails.

Windows and real Android selection, authorization, callback, account eligibility, and PDF inference remain **unverified** at this commit. `adb devices` listed no attached device during implementation. Mocks and successful builds do not clear the live feasibility gate. If either platform fails, preserve sanitized evidence and revisit the connection/platform decision with the user before starting downstream #3. Downstream work also waits for user acceptance and merge of this prerequisite.
