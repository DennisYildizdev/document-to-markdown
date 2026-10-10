import { invoke } from "@tauri-apps/api/core";

export type InputFormat = "text" | "markdown" | "csv" | "tsv" | "json" | "xml";

export interface Capabilities {
  protocolVersion: 1;
  formats: string[];
}

export interface InputSummary {
  inputId: string;
  name: string;
  format: InputFormat;
  size: number;
}

export interface Diagnostic {
  code: string;
  severity: "info" | "warning" | "error";
  message: string;
}

export interface ConversionResult {
  resultId: string;
  markdown: string;
  diagnostics: Diagnostic[];
}

export interface Backend {
  getCapabilities(): Promise<Capabilities>;
  pickInputs(): Promise<InputSummary[]>;
  convertInput(inputId: string): Promise<ConversionResult>;
  exportResult(resultId: string): Promise<string | null>;
}

export const tauriBackend: Backend = {
  getCapabilities: () => invoke("get_capabilities"),
  pickInputs: () => invoke("pick_inputs"),
  convertInput: (inputId) => invoke("convert_input", { inputId }),
  exportResult: (resultId) => invoke("export_result", { resultId }),
};
