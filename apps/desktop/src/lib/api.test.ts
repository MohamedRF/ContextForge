import { describe, expect, it } from "vitest";
import {
  clampBudget,
  clampDepth,
  clampOffset,
  errorMessage,
  normalizePath,
  normalizeQuery,
  normalizeSelector,
} from "./api";

describe("desktop api validation mirrors Rust bounds", () => {
  it("clamps budget, depth and offset to Rust ranges", () => {
    expect(clampBudget(4000)).toBe(4000);
    expect(clampBudget(10)).toBe(256);
    expect(clampBudget(99999)).toBe(32000);
    expect(clampBudget(Number.NaN)).toBe(4000);
    expect(clampBudget(4000.9)).toBe(4000);
    expect(clampDepth(-1)).toBe(0);
    expect(clampDepth(99)).toBe(8);
    expect(clampDepth(Number.NaN)).toBe(2);
    expect(clampOffset(-5)).toBe(0);
    expect(clampOffset(20000)).toBe(10000);
    expect(clampOffset(Number.NaN)).toBe(0);
  });
  it("normalizes queries and selectors", () => {
    expect(normalizeQuery("  hello  ")).toBe("hello");
    expect(normalizeSelector("  Repo::Struct  ")).toBe("Repo::Struct");
    expect(() => normalizeQuery("   ")).toThrow("Query must be 1–256");
    expect(() => normalizeQuery("x".repeat(257))).toThrow(
      "Query must be 1–256",
    );
    expect(() => normalizeQuery("a\x00b")).toThrow("must not contain control");
    expect(() => normalizeSelector("")).toThrow("Selector must be 1–1024");
    expect(() => normalizeSelector("x".repeat(1025))).toThrow(
      "Selector must be 1–1024",
    );
  });
  it("rejects absolute and traversing paths", () => {
    expect(normalizePath("src/main.rs")).toBe("src/main.rs");
    expect(() => normalizePath("")).toThrow("Path must be 1–1024");
    expect(() => normalizePath("/abs/file.rs")).toThrow(
      "must be relative to the repository",
    );
    expect(() => normalizePath("../outside.rs")).toThrow(
      "must be relative to the repository",
    );
    expect(() => normalizePath("C:\\project\\file.rs")).toThrow(
      "must be relative to the repository",
    );
  });
  it("extracts messages from Tauri and plain errors", () => {
    expect(errorMessage(new Error("boom"))).toBe("boom");
    expect(errorMessage("string error")).toBe("string error");
    expect(errorMessage({ message: "object error" })).toBe("object error");
    expect(errorMessage(null)).toBe("Unknown error");
    expect(errorMessage(undefined)).toBe("Unknown error");
    expect(errorMessage(42)).toBe("42");
  });
});
