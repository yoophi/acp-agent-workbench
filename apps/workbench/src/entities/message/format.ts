import type { EventGroup, RunEvent, TimelineItem } from "./model";
import { stripAnsi } from "../../shared/lib";

export function toTimelineItem(runId: string, event: RunEvent): TimelineItem {
  const createdAt = Date.now();
  const base = {
    id: `${runId}-${createdAt}-${Math.random().toString(16).slice(2)}`,
    runId,
    createdAt,
    event,
  };
  const item = buildItem(base, event);
  return { ...item, body: stripAnsi(item.body) };
}

function buildItem(
  base: Pick<TimelineItem, "id" | "runId" | "createdAt" | "event">,
  event: RunEvent,
): TimelineItem {
  switch (event.type) {
    case "agentMessage":
      return { ...base, group: "assistant/message", title: "assistant/message", body: event.text };
    case "thought":
      return { ...base, group: "thought", title: "thought", body: event.text };
    case "plan":
      return {
        ...base,
        group: "lifecycle",
        title: "plan",
        body: event.entries.map((entry) => `${entry.status}: ${entry.content}`).join("\n"),
      };
    case "tool":
      return {
        ...base,
        group: "tool_call/tool_result",
        title: `tool ${event.status}`.trim(),
        body: [event.title, ...event.locations.map((path) => `path: ${path}`)].filter(Boolean).join("\n"),
        tone: event.status === "failed" ? "danger" : event.status === "completed" ? "success" : "info",
      };
    case "usage":
      return {
        ...base,
        group: "usage",
        title: "usage",
        body: `context ${event.used}/${event.size}`,
      };
    case "permission":
      return {
        ...base,
        group: "permission",
        title: "permission",
        body: [
          event.title,
          event.input ? JSON.stringify(event.input) : "",
          event.requiresResponse ? "waiting for approval" : "",
          `selected: ${event.selected ?? "none"}`,
        ]
          .filter(Boolean)
          .join("\n"),
        tone: "warning",
      };
    case "fileSystem":
      return { ...base, group: "tool_call/tool_result", title: `fs.${event.operation}`, body: event.path };
    case "terminal":
      return {
        ...base,
        group: "terminal",
        title: `terminal ${event.operation}`,
        body: [event.terminalId, event.message].filter(Boolean).join("\n"),
      };
    case "diagnostic":
      return { ...base, group: "lifecycle", title: "diagnostic", body: event.message };
    case "lifecycle":
      return {
        ...base,
        group: "lifecycle",
        title: event.status,
        body: event.message,
        tone:
          event.status === "completed" || event.status === "promptCompleted"
            ? "success"
            : event.status === "cancelled"
              ? "warning"
              : "info",
      };
    case "raw":
      return {
        ...base,
        group: "raw",
        title: event.method,
        body: JSON.stringify(event.payload, null, 2),
      };
    case "error":
      return { ...base, group: "error", title: "error", body: event.message, tone: "danger" };
  }
}

export const eventGroups: Array<{ id: EventGroup | "all"; label: string }> = [
  { id: "all", label: "All" },
  { id: "assistant/message", label: "Message" },
  { id: "thought", label: "Thought" },
  { id: "tool_call/tool_result", label: "Tool" },
  { id: "usage", label: "Usage" },
  { id: "permission", label: "Permission" },
  { id: "terminal", label: "Terminal" },
  { id: "lifecycle", label: "Lifecycle" },
  { id: "error", label: "Error" },
  { id: "raw", label: "Raw" },
];
