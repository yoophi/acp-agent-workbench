# Frontend unit tests

Vitest runs TypeScript/React unit tests directly from the source tree. The setup is tuned for **focused feature tests** — targeted tests around pure functions, hooks, and adapter boundaries — rather than broad component snapshots or end-to-end flows.

## Commands

| Script | Purpose |
|---|---|
| `npm run test` | Single-run execution. Used in CI and as a local smoke check. |
| `npm run test:watch` | Watch mode with change re-runs; useful during TDD. |

Tests are picked up by the glob `src/**/*.test.{ts,tsx}`.

## Layout

- `vitest.config.ts` — Vitest + jsdom configuration. Aliases `@/*` to `./src/*`. `@vitejs/plugin-react` is enabled ahead of time so future component tests can import `.tsx` without additional setup; current tests do not exercise JSX. `globals: false` — always `import { describe, it, expect, vi } from "vitest"` explicitly for better IDE navigation and to keep the test style uniform across files.
- `src/test/setup.ts` — Global setup. Polyfills `crypto.randomUUID()` (used by feature code for run/queue ids) and resets fake timers before each test.
- `src/test/tauri.ts` — Test helpers for mocking the Tauri event bridge.

Mocks are auto-reset between tests via `mockReset: true` + `restoreMocks: true` in the config, so individual cases do not need to call `.mockReset()` manually.

## Mocking the Tauri API

`@/shared/api` is the single seam for Tauri IPC (`invokeCommand`, `listenEvent`). Feature tests mock this module rather than the underlying `@tauri-apps/api/*` packages, so tests stay insulated from Tauri internals and do not require a running window.

**Commands** — set per-test behavior with standard Vitest helpers:

```ts
import { describe, expect, it, vi } from "vitest";

vi.mock("../../shared/api", () => ({
  invokeCommand: vi.fn(),
  listenEvent: vi.fn(),
}));

import { invokeCommand } from "../../shared/api";
import { startAgentRun } from "./api";

const mockedInvoke = vi.mocked(invokeCommand);

it("forwards the request to the backend", async () => {
  mockedInvoke.mockResolvedValueOnce({ id: "run-1" });
  await startAgentRun({ goal: "hi", agentId: "claude" });
  expect(mockedInvoke).toHaveBeenCalledWith("start_agent_run", {
    request: { goal: "hi", agentId: "claude" },
  });
});
```

**Events** — use `setupTauriListeners` to route `listenEvent` subscriptions through an in-memory dispatcher so tests can `emit` payloads:

```ts
import { setupTauriListeners } from "../../test/tauri";
import { listenEvent } from "../../shared/api";
import { listenRunEvents } from "./api";

const events = setupTauriListeners(vi.mocked(listenEvent));
const dispose = await listenRunEvents((envelope) => { /* ... */ });
events.emit("agent-run-event", { runId: "run-1", event: { type: "lifecycle" } });
dispose();
```

## Sample tests

- `src/shared/lib/ansi.test.ts` — pure-function smoke test (no mocks, no DOM).
- `src/features/agent-run/api.test.ts` — feature boundary test exercising command forwarding and event subscription against the Tauri mock.

These two tests are intentionally the only ones added with the infrastructure. Broader coverage (tab close orchestration, permission response flow) is tracked in #18 and should follow the same patterns above.

## Non-goals

- No component snapshot tests or end-to-end browser tests.
- No global mock of all Tauri commands; tests wire up only what they exercise so failures point at the offending interaction.
