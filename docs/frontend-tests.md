# Frontend Unit Tests

Vitest는 source tree의 TypeScript/React unit test를 직접 실행한다. 현재 설정은 broad component snapshot이나 end-to-end flow보다, pure function, hook, adapter boundary 주변의 focused feature test에 맞춰져 있다.

## Commands

| Script | 목적 |
| --- | --- |
| `npm run test` | 단일 실행. CI와 local smoke check에서 사용한다. |
| `npm run test:watch` | watch mode. 변경 시 재실행되며 TDD 중 유용하다. |

test file은 `src/**/*.test.{ts,tsx}` glob으로 수집한다.

## Layout

- `vitest.config.ts`: Vitest + jsdom 설정. `@/*` alias를 `./src/*`로 연결한다. 향후 component test가 `.tsx`를 별도 설정 없이 import할 수 있도록 `@vitejs/plugin-react`를 활성화한다. `globals: false`이므로 `describe`, `it`, `expect`, `vi`는 항상 `vitest`에서 명시적으로 import한다.
- `src/test/setup.ts`: global setup. feature code가 run/queue id 생성에 사용하는 `crypto.randomUUID()`를 polyfill하고 각 test 전에 fake timer를 reset한다.
- `src/test/tauri.ts`: Tauri event bridge mock을 위한 test helper.

config의 `mockReset: true`, `restoreMocks: true`로 test 사이 mock이 자동 reset되므로, 개별 case에서 `.mockReset()`을 직접 호출할 필요가 없다.

## Tauri API Mocking

`@/shared/api`는 Tauri IPC의 단일 boundary다. feature test는 underlying `@tauri-apps/api/*` package 대신 이 module을 mock한다. 이렇게 하면 test가 Tauri 내부 구현과 분리되고 실제 window 없이도 실행된다.

### Commands

표준 Vitest helper로 test별 동작을 설정한다.

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

### Events

`setupTauriListeners`를 사용하면 `listenEvent` subscription을 in-memory dispatcher에 연결할 수 있다. test에서는 `emit`으로 payload를 흘려보낸다.

```ts
import { setupTauriListeners } from "../../test/tauri";
import { listenEvent } from "../../shared/api";
import { listenRunEvents } from "./api";

const events = setupTauriListeners(vi.mocked(listenEvent));
const dispose = await listenRunEvents((envelope) => { /* ... */ });
events.emit("agent-run-event", { runId: "run-1", event: { type: "lifecycle" } });
dispose();
```

## Sample Tests

- `src/shared/lib/ansi.test.ts`: pure-function smoke test. mock과 DOM이 필요 없다.
- `src/features/agent-run/api.test.ts`: Tauri mock을 사용해 command forwarding과 event subscription을 검증하는 feature boundary test.

이 두 test는 infrastructure와 함께 추가된 최소 예시다. 더 넓은 coverage, 예를 들어 tab close orchestration이나 permission response flow는 같은 패턴을 따른다.

## Non-goals

- component snapshot test나 end-to-end browser test는 목표가 아니다.
- 모든 Tauri command의 global mock을 만들지 않는다. 각 test가 실제로 사용하는 interaction만 연결해 실패 지점을 명확하게 유지한다.
