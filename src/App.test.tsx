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
});
