import { describe, expect, it, vi } from "vitest";
import { AppWorkflow, UPLOAD_NOTICE, type AiImport, type PdfAcquisition, type PdfDocument } from "./workflow";

const pdf: PdfDocument = { name: "sample.pdf", bytes: new Uint8Array([37, 80, 68, 70, 45]) };
function setup(acquisition: PdfAcquisition = { choosePdf: async () => pdf }, ai: AiImport = { signIn: async () => {}, analyze: async () => "Observed PDF values" }) {
  return { workflow: new AppWorkflow(acquisition, ai), ai };
}
describe("feasibility workflow", () => {
  it("keeps selection local until explicit Analyze PDF", async () => {
    const analyze = vi.fn(async () => "Observed PDF values");
    const { workflow } = setup(undefined, { signIn: async () => {}, analyze });
    await workflow.selectPdf();
    expect(workflow.state.pdf?.name).toBe("sample.pdf");
    expect(analyze).not.toHaveBeenCalled();
    expect(UPLOAD_NOTICE).toContain("sends this PDF to OpenAI");
    await workflow.analyze();
    expect(analyze).toHaveBeenCalledOnce();
    expect(workflow.state.result).toBe("Observed PDF values");
  });
  it("makes acquisition and sign-in failures actionable", async () => {
    const { workflow } = setup({ choosePdf: async () => { throw new Error("permission denied"); } }, { signIn: async () => { throw new Error("callback failed"); }, analyze: async () => "" });
    await workflow.selectPdf(); expect(workflow.state.error).toContain("permission denied");
    await workflow.signIn(); expect(workflow.state.error).toContain("callback failed"); expect(workflow.state.signedIn).toBe(false);
  });
  it("reports granted plan permission separately from successful PDF inference", async () => {
    const { workflow } = setup(); await workflow.signIn();
    expect(workflow.state.signedIn).toBe(true); expect(workflow.state.result).toBeNull();
  });
  it.each(["account not eligible", "provider unavailable", "malformed event", " "])("shows analysis failure: %s", async (result) => {
    const { workflow } = setup(undefined, { signIn: async () => {}, analyze: async () => { if (result.trim()) throw new Error(result); return result; } });
    await workflow.selectPdf(); await workflow.analyze();
    expect(workflow.state.result).toBeNull(); expect(workflow.state.error).toContain("Analysis failed");
  });
  it("requires a selected PDF", async () => {
    const analyze = vi.fn(async () => "unused"); const { workflow } = setup(undefined, { signIn: async () => {}, analyze });
    await workflow.analyze(); expect(workflow.state.error).toContain("Choose a local PDF"); expect(analyze).not.toHaveBeenCalled();
  });
});
