# Frontend Feature-Sliced Design

React frontend는 가벼운 Feature-Sliced Design 구조를 따른다. 목표는 workbench가 커져도 page composition, user-facing feature, business entity, generic infrastructure가 서로 섞이지 않도록 하는 것이다.

## Layers

의존 방향은 위에서 아래로만 흐른다.

```text
app -> pages -> widgets -> features -> entities -> shared
```

- `app`: provider, app-level runtime 설치, routing/layout wiring, global styles.
- `pages`: route-level composition. page는 widget과 feature를 조립하지만 오래 사는 business logic을 소유하지 않는다.
- `widgets`: page section. feature, entity, shared UI로 구성한다. widget은 Tauri command를 직접 호출하거나 long-lived application state를 직접 소유하지 않는다.
- `features`: agent run, goal file loading, permission response, 향후 task/scenario action 같은 사용자 action과 business interaction.
- `entities`: agent, message 같은 안정적인 frontend business object와 formatter.
- `shared`: domain-agnostic infrastructure and generic utility. Shared UI primitives live in the workspace package `@acp/ui`.

## Import Rules

- `app`은 모든 layer를 import할 수 있다.
- `pages`는 `widgets`, `features`, `entities`, `shared`를 import할 수 있다.
- `widgets`는 `features`, `entities`, `shared`를 import할 수 있다.
- `features`는 `entities`, `shared`를 import할 수 있다.
- `entities`는 `shared`를 import할 수 있다.
- `shared`는 app-specific layer를 import하면 안 된다.
- cross-slice import는 가능하면 각 slice의 public `index.ts`를 사용한다.

같은 slice 내부 구현 파일은 relative path를 사용할 수 있다. 예를 들어 `features/agent-run/runtime.ts`는 `./model`을 직접 import할 수 있다.

frontend architecture PR을 열기 전에 boundary check를 실행한다.

```bash
npm run check:fsd
```

이 check는 `src/` 아래의 static import를 검사하고 다음 경우 실패한다.

- 낮은 layer가 높은 layer를 import하는 경우. 예: `features`가 `widgets`를 import.
- `shared`가 app-specific layer를 import하는 경우.
- `entities`, `features`, `widgets`에 대한 cross-slice import가 해당 slice의 public `index.ts` API를 우회하는 경우.

## Public APIs

외부에서 소비되는 각 slice는 `index.ts`를 통해 작은 public API를 노출해야 한다.

예:

- `features/agent-run`
- `features/goal-input`
- `features/permission-response`
- `entities/message`
- `widgets/event-stream`
- `@acp/ui`

다른 slice의 internal file을 직접 import하지 않는다. 예를 들어 widget이나 page에서 `features/agent-run/model.ts`를 직접 import하지 말고, 필요한 symbol은 해당 slice의 `index.ts`에서 명시적으로 export한다.

## Tauri Boundary

`shared/api`는 generic Tauri transport helper만 제공한다.

- `invokeCommand`
- `listenEvent`

typed command wrapper는 use case를 소유하는 feature에 둔다.

- `features/agent-run/api.ts`: agent run command, follow-up prompt, event subscription, agent list.
- `features/goal-input/api.ts`: goal file loading.
- `features/permission-response/api.ts`: permission response command.

이렇게 하면 `shared`가 entity와 feature-specific contract로부터 독립적으로 유지된다.

## Current Slice Ownership

- `features/agent-run`: tab/run state, run lifecycle hook, runtime event listener, follow-up queue draining, tab close orchestration.
- `features/goal-input`: goal editor UI와 goal file loading.
- `features/permission-response`: permission option selection, pending response state, response command.
- `entities/message`: ACP run event type, event formatting, event group.
- `entities/agent`: agent descriptor type.
- `entities/workspace`: 현재 구현의 GitHub-origin repository model. 향후 `WorkContext` taxonomy로 일반화될 수 있다.
