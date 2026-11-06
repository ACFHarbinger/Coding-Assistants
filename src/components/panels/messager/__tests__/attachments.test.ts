import { describe, expect, it } from "vitest";
import {
  isUnsupportedOrRawMime,
  MAX_ATTACHMENT_BYTES,
  uploadAttachment,
} from "../attachments";

describe("attachments guard & validation (#248 QA-11, #247 QA-10)", () => {
  it("rejects files exceeding MAX_ATTACHMENT_BYTES (20 MiB) before reading", async () => {
    const oversizeFile = {
      name: "huge.iso",
      size: 25 * 1024 * 1024, // 25 MiB
      type: "application/octet-stream",
    } as File;

    await expect(uploadAttachment(oversizeFile)).rejects.toThrow(
      /exceeds the 20 MiB limit/,
    );
  });

  it("detects unsupported or raw mime types", () => {
    expect(isUnsupportedOrRawMime("application/octet-stream", "data.bin")).toBe(true);
    expect(isUnsupportedOrRawMime("", "events.jsonl")).toBe(true);
    expect(isUnsupportedOrRawMime("image/png", "photo.png")).toBe(false);
    expect(isUnsupportedOrRawMime("text/plain", "notes.txt")).toBe(false);
  });

  it("defines MAX_ATTACHMENT_BYTES as exactly 20 MiB", () => {
    expect(MAX_ATTACHMENT_BYTES).toBe(20 * 1024 * 1024);
  });
});
