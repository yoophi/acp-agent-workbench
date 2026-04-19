export type RunScenarioId = "default" | "spec-writer";

export type RunScenarioOption = {
  id: RunScenarioId;
  label: string;
  description: string;
};

export const RUN_SCENARIOS: RunScenarioOption[] = [
  {
    id: "default",
    label: "Direct run",
    description: "Send the goal to the selected agent unchanged.",
  },
  {
    id: "spec-writer",
    label: "Spec Writer",
    description: "Generate a Spec-Kit style feature specification from the goal.",
  },
];

type ScenarioPromptContext = {
  workdir?: string | null;
};

export function composeScenarioPrompt(
  scenario: RunScenarioId,
  goal: string,
  context: ScenarioPromptContext = {},
) {
  if (scenario === "default") return goal;
  return composeSpecWriterPrompt(goal, context);
}

function composeSpecWriterPrompt(goal: string, context: ScenarioPromptContext) {
  const workdir = context.workdir?.trim();
  return [
    "You are the Spec Writer for ACP Agent Workbench.",
    "",
    "Create a GitHub Spec-Kit compatible feature specification from the user's feature request.",
    "Focus on WHAT users need and WHY. Do not include implementation details, technology choices, file paths, API designs, database schemas, or task checklists inside spec.md.",
    "",
    "Feature request:",
    goal,
    "",
    "Required output:",
    "- Generate a concise action-noun feature short name, 2-4 words when possible.",
    "- Propose a target spec path using specs/<number>-<short-name>/spec.md. If the next number is unknown, use 001 as a safe placeholder and state that it should be reconciled with existing specs before writing.",
    "- Draft the spec.md content with these sections when relevant: Feature Overview, User Scenarios / User Stories, Acceptance Scenarios, Functional Requirements, Edge Cases, Key Entities, Assumptions, and Success Criteria.",
    "- Functional requirements must use stable IDs such as FR-001, FR-002, and be testable.",
    "- Success criteria must be measurable, user-focused, and technology-agnostic.",
    "- Use informed assumptions for ordinary gaps and list them under Assumptions.",
    "- Ask clarification questions only when no safe assumption exists. Use at most 3 questions, prioritized by scope, security/privacy, then user experience.",
    "- Keep the response ready for a later plan/tasks workflow; do not generate plan.md or tasks.md.",
    "",
    "Quality checklist to apply before finalizing:",
    "- No implementation details in spec.md.",
    "- Requirements are unambiguous and testable.",
    "- Success criteria are measurable and technology-agnostic.",
    "- Primary user scenarios and edge cases are covered.",
    "- No unresolved placeholders remain except approved clarification markers.",
    "",
    workdir ? `Workspace context: ${workdir}` : "Workspace context: not selected.",
  ].join("\n");
}
