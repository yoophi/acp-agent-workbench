import { useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { eventGroups } from "../../entities/message/format";
import type { EventGroup, TimelineItem } from "../../entities/message/model";
import { respondAgentPermission } from "../../shared/api/tauri";
import { cn } from "../../shared/lib/utils";
import { Button } from "../../shared/ui/Button";
import { Card, CardHeader, CardTitle, CardTitleBlock } from "../../shared/ui/Card";

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
    <Card as="section" className="grid h-full min-h-0 grid-rows-[auto_minmax(0,1fr)] max-lg:h-[620px]" aria-labelledby="stream-heading">
      <CardHeader className="items-start">
        <CardTitleBlock>
          <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">Stream</p>
          <CardTitle id="stream-heading">ACP messages</CardTitle>
        </CardTitleBlock>
        <div className="flex max-w-[620px] flex-wrap justify-end gap-1.5 max-sm:justify-start" role="tablist" aria-label="Event filter">
          {eventGroups.map((group) => (
            <Button
              key={group.id}
              size="sm"
              variant={group.id === filter ? "default" : "outline"}
              type="button"
              onClick={() => onFilterChange(group.id)}
            >
              {group.label}
            </Button>
          ))}
        </div>
      </CardHeader>

      <div className="min-h-0 overflow-auto p-4" role="log" aria-live="polite">
        {items.length === 0 ? (
          <div className="grid min-h-[260px] place-items-center rounded-lg border border-dashed bg-muted/30 text-muted-foreground">
            No ACP messages yet.
          </div>
        ) : (
          items.map((item) => (
            <article
              key={item.id}
              className={cn("mt-2.5 grid grid-cols-[132px_minmax(0,1fr)] gap-3.5 rounded-md border-l-4 bg-background px-3.5 py-3 first:mt-0 max-sm:grid-cols-1", toneClassName(item))}
            >
              <div className="grid content-start gap-1">
                <span className="text-xs font-semibold uppercase text-muted-foreground">{item.group}</span>
                <strong className="[overflow-wrap:anywhere] text-sm font-medium">{item.title}</strong>
              </div>
              {isMarkdownStream(item) ? (
                <StreamingMarkdown content={item.body} />
              ) : (
                <pre className="m-0 min-w-0 whitespace-pre-wrap break-words font-mono text-sm leading-6">
                  {item.body}
                </pre>
              )}
              {item.event.type === "permission" && item.event.requiresResponse && item.event.permissionId ? (
                <div className="col-start-2 mt-[-4px] flex gap-2 max-sm:col-start-1">
                  <Button
                    type="button"
                    variant="primary"
                    size="sm"
                    onClick={() => respondToPermission(item, "allow", setPendingPermissionIds, onError)}
                    disabled={pendingPermissionIds.has(item.event.permissionId) || !findPermissionOption(item, "allow")}
                  >
                    Approve
                  </Button>
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    onClick={() => respondToPermission(item, "reject", setPendingPermissionIds, onError)}
                    disabled={pendingPermissionIds.has(item.event.permissionId) || !findPermissionOption(item, "reject")}
                  >
                    Reject
                  </Button>
                </div>
              ) : null}
            </article>
          ))
        )}
        <div ref={endRef} />
      </div>
    </Card>
  );
}

function isMarkdownStream(item: TimelineItem) {
  return item.group === "assistant/message" || item.group === "thought";
}

function StreamingMarkdown({ content }: { content: string }) {
  return (
    <div className="min-w-0 break-words text-sm leading-6">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children, ...props }) => (
            <h1 className="mb-2 mt-4 text-2xl font-semibold tracking-tight first:mt-0" {...props}>
              {children}
            </h1>
          ),
          h2: ({ children, ...props }) => (
            <h2 className="mb-2 mt-4 text-xl font-semibold tracking-tight first:mt-0" {...props}>
              {children}
            </h2>
          ),
          h3: ({ children, ...props }) => (
            <h3 className="mb-2 mt-3 text-lg font-semibold tracking-tight first:mt-0" {...props}>
              {children}
            </h3>
          ),
          h4: ({ children, ...props }) => (
            <h4 className="mb-2 mt-3 text-base font-semibold tracking-tight first:mt-0" {...props}>
              {children}
            </h4>
          ),
          p: ({ children, ...props }) => (
            <p className="my-2 first:mt-0 last:mb-0" {...props}>
              {children}
            </p>
          ),
          ul: ({ children, ...props }) => (
            <ul className="my-2 list-disc pl-5" {...props}>
              {children}
            </ul>
          ),
          ol: ({ children, ...props }) => (
            <ol className="my-2 list-decimal pl-5" {...props}>
              {children}
            </ol>
          ),
          li: ({ children, ...props }) => (
            <li className="mt-1" {...props}>
              {children}
            </li>
          ),
          blockquote: ({ children, ...props }) => (
            <blockquote className="my-3 border-l-4 pl-4 text-muted-foreground" {...props}>
              {children}
            </blockquote>
          ),
          code: ({ children, className, ...props }) => (
            <code className={cn("rounded bg-muted px-1.5 py-0.5 font-mono text-[0.92em]", className)} {...props}>
              {children}
            </code>
          ),
          pre: ({ children, ...props }) => (
            <pre className="my-3 overflow-x-auto rounded-md border bg-muted p-3" {...props}>
              {children}
            </pre>
          ),
          a: ({ children, ...props }) => (
            <a className="font-medium text-primary underline underline-offset-4" rel="noreferrer" target="_blank" {...props}>
              {children}
            </a>
          ),
          hr: (props) => <hr className="my-4 border-border" {...props} />,
          table: ({ children, ...props }) => (
            <div className="my-3 overflow-x-auto">
              <table className="w-full border-collapse text-sm" {...props}>
                {children}
              </table>
            </div>
          ),
          th: ({ children, ...props }) => (
            <th className="border bg-muted px-2 py-1 text-left font-semibold align-top" {...props}>
              {children}
            </th>
          ),
          td: ({ children, ...props }) => (
            <td className="border px-2 py-1 align-top" {...props}>
              {children}
            </td>
          ),
          img: ({ alt, ...props }) => <img className="h-auto max-w-full rounded-md" alt={alt ?? ""} {...props} />,
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

function toneClassName(item: TimelineItem) {
  switch (item.tone) {
    case "success":
      return "border-l-primary";
    case "warning":
      return "border-l-amber-500";
    case "danger":
      return "border-l-destructive";
    case "info":
    default:
      return "border-l-sky-600";
  }
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
