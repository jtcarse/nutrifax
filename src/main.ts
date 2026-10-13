import "./styles.css";
import { AppWorkflow, UPLOAD_NOTICE } from "./workflow";
import { nativePdfAcquisition, chatGptImport } from "./adapters";
const workflow = new AppWorkflow(nativePdfAcquisition, chatGptImport);
const app = document.querySelector<HTMLDivElement>("#app")!;
function render() {
  const { pdf, signedIn, result, error, busy } = workflow.state;
  app.innerHTML = `<h1>Nutrifax feasibility</h1><p>Select a local text-based nutrition PDF, connect an eligible ChatGPT account, then analyze it.</p>
    <section><h2>1. PDF</h2><button id="select" ${busy ? "disabled" : ""}>Choose PDF</button><p id="selected"></p></section>
    <section><h2>2. ChatGPT</h2><button id="sign-in" ${busy ? "disabled" : ""}>Continue with ChatGPT</button><p>${signedIn ? "ChatGPT plan permission granted for this session. PDF inference is still unverified until Analyze PDF succeeds." : "Sign in and grant ChatGPT plan usage. Signing in alone does not establish PDF inference access."}</p></section>
    <section><h2>3. Analyze</h2><p class="notice">${UPLOAD_NOTICE}</p><button id="analyze" ${busy || !pdf ? "disabled" : ""}>Analyze PDF</button></section>
    <p role="status">${busy ? "Working…" : ""}</p><p role="alert" id="error"></p><section><h2>Inference result</h2><pre id="result"></pre></section>`;
  app.querySelector("#selected")!.textContent = pdf ? `${pdf.name} selected locally (${pdf.bytes.length} bytes).` : "No PDF selected.";
  app.querySelector("#error")!.textContent = error || "";
  app.querySelector("#result")!.textContent = result || "No inference result yet.";
  app.querySelector("#select")!.addEventListener("click", async () => { await workflow.selectPdf(); render(); });
  app.querySelector("#sign-in")!.addEventListener("click", () => { renderBusy(workflow.signIn()); });
  app.querySelector("#analyze")!.addEventListener("click", () => { renderBusy(workflow.analyze()); });
}
async function renderBusy(operation: Promise<void>) { render(); await operation; render(); }
render();
