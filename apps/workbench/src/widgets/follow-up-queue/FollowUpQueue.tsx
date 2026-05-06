import { X } from "lucide-react";
import type { FollowUpQueueItem } from "../../features/agent-run";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle, CardTitleBlock } from "../../shared/ui";

type FollowUpQueueProps = {
  items: FollowUpQueueItem[];
  awaitingResponse: boolean;
  onCancel: (id: string) => void;
};

export function FollowUpQueue({ items, awaitingResponse, onCancel }: FollowUpQueueProps) {
  if (items.length === 0) {
    return null;
  }

  return (
    <Card as="section" aria-labelledby="follow-up-queue-heading">
      <CardHeader>
        <CardTitleBlock>
          <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">Queue</p>
          <CardTitle id="follow-up-queue-heading">Pending follow-ups</CardTitle>
        </CardTitleBlock>
        <Badge variant="secondary" aria-label={`${items.length} queued`}>
          {items.length}
        </Badge>
      </CardHeader>
      <CardContent>
        <ul className="m-0 flex list-none flex-col gap-2 p-0">
          {items.map((item, index) => (
            <li key={item.id} className="flex items-start gap-3 rounded-lg border bg-muted/30 p-3">
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  {index + 1}
                  {index === 0 && awaitingResponse ? " · next" : ""}
                </span>
                <p className="m-0 whitespace-pre-wrap break-words text-sm leading-6">
                  {item.text}
                </p>
              </div>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-muted-foreground hover:text-destructive"
                aria-label="Remove from queue"
                onClick={() => onCancel(item.id)}
              >
                <X size={14} />
              </Button>
            </li>
          ))}
        </ul>
      </CardContent>
    </Card>
  );
}
