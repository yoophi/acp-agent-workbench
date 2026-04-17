import { describe, expect, it } from "vitest";
import { stripAnsi } from "./ansi";

describe("stripAnsi", () => {
  it("returns plain text unchanged", () => {
    expect(stripAnsi("hello world")).toBe("hello world");
  });

  it("removes SGR color escape sequences", () => {
    expect(stripAnsi("\u001B[31merror\u001B[0m")).toBe("error");
  });

  it("removes cursor control sequences", () => {
    expect(stripAnsi("progress\u001B[2K\u001B[Gdone")).toBe("progressdone");
  });
});
