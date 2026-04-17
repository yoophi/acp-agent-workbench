# Frontend Feature-Sliced Design

The React frontend follows a lightweight Feature-Sliced Design layout. The goal is to keep page composition, user-facing features, business entities, and generic infrastructure from bleeding into each other as the workbench grows.

## Layers

Dependency direction flows from top to bottom:

```text
app -> pages -> widgets -> features -> entities -> shared
```

- `app`: providers, app-level runtime installation, routing/layout wiring, global styles.
- `pages`: route-level composition. Pages assemble widgets and features but should not own durable business logic.
- `widgets`: page sections made from features, entities, and shared UI. Widgets should not call Tauri commands or own long-lived application state directly.
- `features`: user actions and business interactions, such as agent runs, goal file loading, permission responses, and future task/scenario actions.
- `entities`: stable frontend business objects and formatting helpers, such as agents and messages.
- `shared`: domain-agnostic infrastructure, reusable UI primitives, and generic utilities.

## Import Rules

- `app` may import from any layer.
- `pages` may import from `widgets`, `features`, `entities`, and `shared`.
- `widgets` may import from `features`, `entities`, and `shared`.
- `features` may import from `entities` and `shared`.
- `entities` may import from `shared`.
- `shared` must not import from app-specific layers.
- Cross-slice imports should use each slice's public `index.ts` when practical.

Same-slice imports can use relative paths for internal implementation files. For example, `features/agent-run/runtime.ts` can import `./model` directly.

Run the automated boundary check before opening frontend architecture PRs:

```bash
npm run check:fsd
```

The check validates local static imports under `src/` and fails when:

- a lower layer imports upward, such as `features` importing `widgets`;
- `shared` imports an app-specific layer;
- a cross-slice import into `entities`, `features`, or `widgets` bypasses that slice's public `index.ts` API.

## Public APIs

Each externally consumed slice should expose a small public API through `index.ts`.

Examples:

- `features/agent-run`
- `features/goal-input`
- `features/permission-response`
- `entities/message`
- `widgets/event-stream`
- `shared/ui`

Avoid importing another slice's internal files directly, such as `features/agent-run/model.ts`, from widgets or pages unless the symbol is deliberately exported by that slice.

## Tauri Boundary

`shared/api` exposes generic Tauri transport helpers only:

- `invokeCommand`
- `listenEvent`

Typed command wrappers belong to the feature that owns the use case:

- `features/agent-run/api.ts`: agent run commands, follow-up prompt, event subscription, agent list.
- `features/goal-input/api.ts`: goal file loading.
- `features/permission-response/api.ts`: permission response command.

This keeps `shared` independent from entities and feature-specific contracts.

## Current Slice Ownership

- `features/agent-run`: tab/run state, run lifecycle hook, runtime event listener, follow-up queue draining, tab close orchestration.
- `features/goal-input`: goal editor UI and goal file loading.
- `features/permission-response`: permission option selection, pending response state, response command.
- `entities/message`: ACP run event types, event formatting, event groups.
- `entities/agent`: agent descriptor type.
- `widgets/*`: visual workbench sections that compose feature APIs and shared UI.

## Review Checklist

- Does the changed file import only from allowed lower layers?
- Are cross-slice imports going through public APIs?
- Does a widget call transport/API code directly? If yes, move that behavior into a feature.
- Does `shared` import an entity, feature, widget, page, or app module? If yes, move the domain-specific wrapper out of `shared`.
- Does a broad feature store own unrelated state that should become an entity or a smaller feature concern?
