import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import type { Backend, InputSummary } from "./lib/backend";

const input: InputSummary = {
  inputId: "input-1",
  name: "notes.txt",
  format: "text",
  size: 12,
};

describe("document conversion workflow", () => {
  it("adds files, converts a selected input, and previews Markdown", async () => {
    const backend: Backend = {
      getCapabilities: vi.fn().mockResolvedValue({ protocolVersion: 1, formats: ["txt", "md"] }),
      pickInputs: vi.fn().mockResolvedValue([input]),
      convertInput: vi.fn().mockResolvedValue({
        resultId: "result-1",
        markdown: "Hello from Rust\n",
        diagnostics: [],
      }),
      exportResult: vi.fn(),
    };

    render(<App backend={backend} />);

    fireEvent.click(screen.getByRole("button", { name: /add files/i }));
    expect((await screen.findAllByText("notes.txt")).length).toBeGreaterThan(0);

    fireEvent.click(screen.getByRole("button", { name: /^convert$/i }));
    expect(await screen.findByText("Hello from Rust")).toBeInTheDocument();
    expect(backend.convertInput).toHaveBeenCalledWith("input-1");
  });

  it("shows conversion diagnostics beside the preview", async () => {
    const backend: Backend = {
      getCapabilities: vi.fn().mockResolvedValue({ protocolVersion: 1, formats: ["csv"] }),
      pickInputs: vi.fn().mockResolvedValue([{ ...input, name: "ragged.csv", format: "csv" }]),
      convertInput: vi.fn().mockResolvedValue({
        resultId: "result-2",
        markdown: "| Column 1 | Column 2 |\n| --- | --- |\n| a |  |\n",
        diagnostics: [{
          code: "RAGGED_ROWS",
          severity: "warning",
          message: "Rows contain different field counts; missing cells were padded.",
        }],
      }),
      exportResult: vi.fn(),
    };

    render(<App backend={backend} />);
    fireEvent.click(screen.getByRole("button", { name: /add files/i }));
    expect((await screen.findAllByText("ragged.csv")).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: /^convert$/i }));

    expect(await screen.findByText("RAGGED_ROWS")).toBeInTheDocument();
    expect(screen.getByText(/missing cells were padded/i)).toBeInTheDocument();
  });
});
