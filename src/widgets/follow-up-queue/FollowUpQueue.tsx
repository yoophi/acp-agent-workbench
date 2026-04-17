import { X } from "lucide-react";
import type { FollowUpQueueItem } from "../../features/agent-run/model";

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
    <section className="panel follow-up-queue" aria-labelledby="follow-up-queue-heading">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">Queue</p>
          <h2 id="follow-up-queue-heading">Pending follow-ups</h2>
        </div>
        <span className="queue-count" aria-label={`${items.length} queued`}>
          {items.length}
        </span>
      </div>
      <ul className="queue-list">
        {items.map((item, index) => (
          <li key={item.id} className="queue-item">
            <div className="queue-body">
              <span className="queue-index">
                {index + 1}
                {index === 0 && awaitingResponse ? " · next" : ""}
              </span>
              <p className="queue-text">{item.text}</p>
            </div>
            <button
              type="button"
              className="queue-cancel"
              aria-label="Remove from queue"
              onClick={() => onCancel(item.id)}
            >
              <X size={14} />
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
