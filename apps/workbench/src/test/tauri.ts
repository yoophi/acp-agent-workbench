import type { Mock } from "vitest";

/**
 * Feature-test helper for the Tauri event bridge.
 *
 * The `@/shared/api` module exposes `invokeCommand` (commands) and
 * `listenEvent` (events). For commands, tests typically call
 * `mockResolvedValueOnce` / `mockRejectedValueOnce` on the mock directly.
 * Event dispatch needs a small registry so tests can push payloads
 * to subscribed listeners — that is what this helper provides.
 *
 * Usage:
 * ```ts
 * vi.mock("@/shared/api", () => ({
 *   invokeCommand: vi.fn(),
 *   listenEvent: vi.fn(),
 * }));
 *
 * import { invokeCommand, listenEvent } from "@/shared/api";
 * import { setupTauriListeners } from "@/test/tauri";
 *
 * const events = setupTauriListeners(vi.mocked(listenEvent));
 * events.emit("agent-run-event", payload);
 * ```
 */
export function setupTauriListeners(mockListen: Mock) {
  const listeners = new Map<string, Set<(payload: unknown) => void>>();

  mockListen.mockImplementation(async (eventName: string, callback: (payload: unknown) => void) => {
    const existing = listeners.get(eventName);
    const target = existing ?? new Set<(payload: unknown) => void>();
    if (!existing) {
      listeners.set(eventName, target);
    }
    target.add(callback);
    return () => {
      target.delete(callback);
    };
  });

  return {
    /** Deliver `payload` to every listener currently subscribed to `eventName`. */
    emit(eventName: string, payload: unknown) {
      listeners.get(eventName)?.forEach((cb) => cb(payload));
    },
    /** Number of active listeners for `eventName`. */
    count(eventName: string) {
      return listeners.get(eventName)?.size ?? 0;
    },
    /** Drop every listener, as if all dispose handles were invoked. */
    reset() {
      listeners.clear();
    },
  };
}
