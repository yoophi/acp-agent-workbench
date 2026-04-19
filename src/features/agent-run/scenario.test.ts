import { describe, expect, it } from "vitest";
import { composeScenarioPrompt } from "./scenario";

describe("run scenarios", () => {
  it("keeps the default scenario prompt unchanged", () => {
    expect(composeScenarioPrompt("default", "Ship the feature")).toBe("Ship the feature");
  });

  it("wraps spec writer runs in Spec-Kit style guidance", () => {
    const prompt = composeScenarioPrompt("spec-writer", "Add workspace task orchestration", {
      workdir: "/repo/workbench",
    });

    expect(prompt).toContain("Spec Writer");
    expect(prompt).toContain("Feature request:\nAdd workspace task orchestration");
    expect(prompt).toContain("Functional requirements must use stable IDs such as FR-001");
    expect(prompt).toContain("at most 3 questions");
    expect(prompt).toContain("Do not include implementation details");
    expect(prompt).toContain("Workspace context: /repo/workbench");
  });
});
