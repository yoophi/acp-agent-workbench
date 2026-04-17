import { Send } from "lucide-react";
import type { KeyboardEvent } from "react";
import { Button } from "../../shared/ui/Button";

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

  const sendLabel =
    awaitingResponse || queueLength > 0 ? "Queue" : "Send";

  return (
    <section className="panel follow-up-panel" aria-labelledby="follow-up-heading">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">Follow-up</p>
          <h2 id="follow-up-heading">Ask more</h2>
        </div>
        <span className="hint">⌘/Ctrl + Enter</span>
      </div>
      <textarea
        className="follow-up-textarea"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={!sessionActive}
        spellCheck={false}
      />
      <div className="follow-up-actions">
        <Button type="button" variant="primary" icon={<Send size={16} />} disabled={!canSubmit} onClick={onSend}>
          {sendLabel}
        </Button>
      </div>
    </section>
  );
}
