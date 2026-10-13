import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { open } from "@tauri-apps/plugin-dialog";
import { readFile } from "@tauri-apps/plugin-fs";
import type { PdfAcquisition, PdfDocument, AiImport } from "./workflow";

export const nativePdfAcquisition: PdfAcquisition = {
  async choosePdf() {
    const path = await open({ multiple: false, directory: false, filters: [{ name: "PDF", extensions: ["pdf"] }] });
    if (!path) return null;
    const bytes = await readFile(path);
    if (bytes.length > 10 * 1024 * 1024) throw new Error("PDF exceeds the 10 MB feasibility limit");
    if (bytes.length < 5 || String.fromCharCode(...bytes.slice(0, 5)) !== "%PDF-") throw new Error("Selected file is not a PDF");
    return { name: path.split(/[\\/]/).pop() || "nutrition.pdf", bytes };
  },
};
export const chatGptImport: AiImport = {
  async signIn() {
    const url = await invoke<string>("begin_sign_in");
    await openUrl(url);
    const deadline = Date.now() + 120_000;
    while (Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      const status = await invoke<{ state: string; message?: string }>("sign_in_status");
      if (status.state === "ready") return;
      if (status.state === "error") throw new Error(status.message || "Authorization failed");
    }
    throw new Error("Timed out waiting for the browser callback");
  },
  async analyze(pdf: PdfDocument) {
    let data = "";
    for (let offset = 0; offset < pdf.bytes.length; offset += 8192) {
      data += String.fromCharCode(...pdf.bytes.subarray(offset, offset + 8192));
    }
    return await invoke<string>("analyze_pdf", { name: pdf.name, data: btoa(data) });
  },
};
