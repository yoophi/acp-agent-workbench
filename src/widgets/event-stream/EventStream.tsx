import { useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { eventGroups } from "../../entities/message/format";
import type { EventGroup, TimelineItem } from "../../entities/message/model";
import { respondAgentPermission } from "../../shared/api/tauri";
import { classNames } from "../../shared/lib/classNames";

type EventStreamProps = {
  items: TimelineItem[];
  filter: EventGroup | "all";
  onFilterChange: (filter: EventGroup | "all") => void;
  onError: (message: string | null) => void;
};

export function EventStream({ items, filter, onFilterChange, onError }: EventStreamProps) {
  const endRef = useRef<HTMLDivElement | null>(null);
  const [pendingPermissionIds, setPendingPermissionIds] = useState<Set<string>>(() => new Set());

  useEffect(() => {
    endRef.current?.scrollIntoView({ block: "end" });
  }, [items]);

  useEffect(() => {
    setPendingPermissionIds((current) => {
      const next = new Set(current);
      for (const item of items) {
        if (item.event.type === "permission" && item.event.permissionId && !item.event.requiresResponse) {
          next.delete(item.event.permissionId);
        }
      }
      return next;
    });
  }, [items]);

  return (
    <section className="panel stream-panel" aria-labelledby="stream-heading">
      <div className="panel-heading stream-heading">
        <div>
          <p className="eyebrow">Stream</p>
          <h2 id="stream-heading">ACP messages</h2>
        </div>
        <div className="segmented" role="tablist" aria-label="Event filter">
          {eventGroups.map((group) => (
            <button
              key={group.id}
              className={group.id === filter ? "selected" : ""}
              type="button"
              onClick={() => onFilterChange(group.id)}
            >
              {group.label}
            </button>
          ))}
        </div>
      </div>

      <div className="timeline" role="log" aria-live="polite">
        {items.length === 0 ? (
          <div className="empty-state">No ACP messages yet.</div>
        ) : (
          items.map((item) => (
            <article key={item.id} className={classNames("timeline-item", item.tone && `tone-${item.tone}`)}>
              <div className="timeline-label">
                <span>{item.group}</span>
                <strong>{item.title}</strong>
              </div>
              {item.group === "assistant/message" ? <StreamingMarkdown content={item.body} /> : <pre>{item.body}</pre>}
              {item.event.type === "permission" && item.event.requiresResponse && item.event.permissionId ? (
                <div className="permission-actions">
                  <button
                    type="button"
                    onClick={() => respondToPermission(item, "allow", setPendingPermissionIds, onError)}
                    disabled={pendingPermissionIds.has(item.event.permissionId) || !findPermissionOption(item, "allow")}
                  >
                    Approve
                  </button>
                  <button
                    type="button"
                    onClick={() => respondToPermission(item, "reject", setPendingPermissionIds, onError)}
                    disabled={pendingPermissionIds.has(item.event.permissionId) || !findPermissionOption(item, "reject")}
                  >
                    Reject
                  </button>
                </div>
              ) : null}
            </article>
          ))
        )}
        <div ref={endRef} />
      </div>
    </section>
  );
}

function StreamingMarkdown({ content }: { content: string }) {
  return (
    <div className="markdown-stream">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          pre: ({ children, ...props }) => (
            <pre className="markdown-code" {...props}>
              {children}
            </pre>
          ),
          a: ({ children, ...props }) => (
            <a rel="noreferrer" target="_blank" {...props}>
              {children}
            </a>
          ),
        }}
      >
        {normalizeStreamingMarkdown(content)}
      </ReactMarkdown>
    </div>
  );
}

function normalizeStreamingMarkdown(content: string) {
  const fenceMatches = content.match(/```/g);
  if (fenceMatches && fenceMatches.length % 2 === 1) {
    const suffix = content.endsWith("\n") ? "```" : "\n```";
    return `${content}${suffix}`;
  }
  return content;
}

function findPermissionOption(item: TimelineItem, mode: "allow" | "reject") {
  if (item.event.type !== "permission") {
    return undefined;
  }
  return item.event.options.find((option) => {
    const kind = option.kind.toLowerCase();
    if (mode === "allow") {
      return kind.startsWith("allow");
    }
    return kind.startsWith("reject") || kind.startsWith("deny");
  });
}

async function respondToPermission(
  item: TimelineItem,
  mode: "allow" | "reject",
  setPendingPermissionIds: Dispatch<SetStateAction<Set<string>>>,
  onError: (message: string | null) => void,
) {
  if (item.event.type !== "permission" || !item.event.permissionId) {
    return;
  }
  const option = findPermissionOption(item, mode);
  if (!option) {
    return;
  }
  const permissionId = item.event.permissionId;
  setPendingPermissionIds((current) => new Set(current).add(permissionId));
  try {
    await respondAgentPermission(permissionId, option.optionId);
    onError(null);
  } catch (err) {
    setPendingPermissionIds((current) => {
      const next = new Set(current);
      next.delete(permissionId);
      return next;
    });
    onError(String(err));
  }
}
