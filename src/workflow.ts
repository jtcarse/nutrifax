export interface PdfDocument { name: string; bytes: Uint8Array }
export interface PdfAcquisition { choosePdf(): Promise<PdfDocument | null> }
export interface AiImport { signIn(): Promise<void>; analyze(pdf: PdfDocument): Promise<string> }
export type WorkflowState = { pdf: PdfDocument | null; signedIn: boolean; result: string | null; error: string | null; busy: boolean };
export const UPLOAD_NOTICE = "Analyze PDF sends this PDF to OpenAI through your ChatGPT connection.";

export class AppWorkflow {
  readonly state: WorkflowState = { pdf: null, signedIn: false, result: null, error: null, busy: false };
  constructor(private readonly acquisition: PdfAcquisition, private readonly ai: AiImport) {}

  async selectPdf(): Promise<void> {
    this.state.error = null; this.state.result = null;
    try { const pdf = await this.acquisition.choosePdf(); if (pdf) this.state.pdf = pdf; }
    catch (error) { this.state.pdf = null; this.state.error = `Could not read PDF: ${message(error)}. Choose a readable text-based PDF and retry.`; }
  }
  async signIn(): Promise<void> {
    this.state.error = null; this.state.busy = true;
    try { await this.ai.signIn(); this.state.signedIn = true; }
    catch (error) { this.state.signedIn = false; this.state.error = `ChatGPT sign-in failed: ${message(error)}. Retry sign-in in the system browser.`; }
    finally { this.state.busy = false; }
  }
  async analyze(): Promise<void> {
    this.state.error = null; this.state.result = null;
    if (!this.state.pdf) { this.state.error = "Choose a local PDF before analysis."; return; }
    this.state.busy = true;
    try { const result = await this.ai.analyze(this.state.pdf); if (!result.trim()) throw new Error("OpenAI returned an empty result"); this.state.result = result; }
    catch (error) { this.state.error = `Analysis failed: ${message(error)}. Check your ChatGPT plan access and try again.`; }
    finally { this.state.busy = false; }
  }
}
function message(error: unknown): string { return error instanceof Error ? error.message : String(error); }
