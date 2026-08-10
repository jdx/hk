import { describe, expect, it } from "vitest";
import { structuredContent } from "./bridge";

describe("MCP Apps bridge helpers", () => {
  it("extracts structured content and tolerates fallback results", () => {
    expect(
      structuredContent({ structuredContent: { run: { id: "hk-1" } } }),
    ).toEqual({ run: { id: "hk-1" } });
    expect(structuredContent(undefined)).toBeUndefined();
  });
});
