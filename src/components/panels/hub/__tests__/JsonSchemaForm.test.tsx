import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import {
  emptyArgsFromSchema,
  JsonSchemaForm,
  preparedArguments,
  type JsonSchema,
} from "../JsonSchemaForm";

const askSchema: JsonSchema = {
  type: "object",
  properties: {
    query: { type: "string", description: "Question to ask" },
    recency: { type: "string", enum: ["day", "week", "month"] },
    count: { type: "integer" },
  },
  required: ["query"],
};

describe("JsonSchemaForm", () => {
  it("renders generated fields from a tools/list inputSchema", () => {
    const value = emptyArgsFromSchema(askSchema);
    render(<JsonSchemaForm schema={askSchema} value={value} onChange={() => undefined} />);
    expect(screen.getByText(/query/i)).toBeTruthy();
    expect(screen.getByText(/Question to ask/)).toBeTruthy();
    expect(screen.getByPlaceholderText("query")).toBeTruthy();
    expect(screen.getByText("day")).toBeTruthy();
  });

  it("omits blank optional fields and keeps required ones", () => {
    const prepared = preparedArguments(askSchema, {
      query: "latest rust news",
      recency: "",
      count: "",
    });
    expect(prepared).toEqual({ query: "latest rust news" });
  });

  it("rejects a missing required argument", () => {
    expect(() => preparedArguments(askSchema, { query: "" })).toThrow(/query/);
  });
});
