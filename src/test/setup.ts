import { beforeEach, vi } from "vitest";

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });

// Provide crypto.randomUUID() in the jsdom environment where it is
// absent by default. Feature code calls it when allocating run ids and
// queue item ids, and the real implementation is deterministic enough
// for test scenarios that do not assert on the generated id.
if (typeof globalThis.crypto === "undefined" || typeof globalThis.crypto.randomUUID !== "function") {
  let counter = 0;
  const fallback = {
    randomUUID: () => {
      counter += 1;
      return `00000000-0000-0000-0000-${counter.toString().padStart(12, "0")}`;
    },
  };
  Object.defineProperty(globalThis, "crypto", {
    configurable: true,
    value: fallback,
  });
}

beforeEach(() => {
  vi.useRealTimers();
});
