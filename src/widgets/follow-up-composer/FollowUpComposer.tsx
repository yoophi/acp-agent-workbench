import { Send } from "lucide-react";
import type { KeyboardEvent } from "react";
import { Button, Card, CardContent, CardHeader, CardTitle, CardTitleBlock, Textarea } from "../../shared/ui";

type FollowUpComposerProps = {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  sessionActive: boolean;
  awaitingResponse: boolean;
  queueLength: number;
};

export function FollowUpComposer({
  value,
  onChange,
  onSend,
  sessionActive,
  awaitingResponse,
  queueLength,
}: FollowUpComposerProps) {
  const canSubmit = sessionActive && value.trim().length > 0;

  function handleKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if ((event.metaKey || event.ctrlKey) && event.key === "Enter" && canSubmit) {
      event.preventDefault();
      onSend();
    }
  }

  const placeholder = sessionActive
    ? awaitingResponse || queueLength > 0
      ? "The agent is busy. Send will queue this prompt."
      : "Send an additional instruction to the running agent."
    : "Start a run to send follow-up prompts.";

  const sendLabel = awaitingResponse || queueLength > 0 ? "Queue" : "Send";

  return (
    <Card as="section" aria-labelledby="follow-up-heading">
      <CardHeader>
        <CardTitleBlock>
          <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">Follow-up</p>
          <CardTitle id="follow-up-heading">Ask more</CardTitle>
        </CardTitleBlock>
        <span className="text-xs font-medium text-muted-foreground">⌘/Ctrl + Enter</span>
      </CardHeader>
      <CardContent className="grid gap-3 pt-6">
        <Textarea
          className="min-h-[110px] resize-y"
          value={value}
          onChange={(event) => onChange(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          disabled={!sessionActive}
          spellCheck={false}
        />
        <div className="flex justify-end">
          <Button type="button" variant="primary" icon={<Send size={16} />} disabled={!canSubmit} onClick={onSend}>
            {sendLabel}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
