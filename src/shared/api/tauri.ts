import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export function invokeCommand<T>(command: string, args?: Record<string, unknown>) {
  return invoke<T>(command, args);
}

export function listenEvent<T>(eventName: string, callback: (payload: T) => void) {
  return listen<T>(eventName, (event) => callback(event.payload));
}
