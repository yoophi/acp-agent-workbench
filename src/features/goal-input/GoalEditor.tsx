import { open } from "@tauri-apps/plugin-dialog";
import { FileUp } from "lucide-react";
import { loadGoalFile } from "../../shared/api/tauri";
import { Button } from "../../shared/ui/Button";

type GoalEditorProps = {
  value: string;
  onChange: (value: string) => void;
  onError: (value: string | null) => void;
};

export function GoalEditor({ value, onChange, onError }: GoalEditorProps) {
  async function handleLoadFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Goal text", extensions: ["txt", "md"] }],
    });
    if (!selected || Array.isArray(selected)) {
      return;
    }
    try {
      onChange(await loadGoalFile(selected));
      onError(null);
    } catch (err) {
      onError(String(err));
    }
  }

  return (
    <section className="panel goal-panel" aria-labelledby="goal-heading">
      <div className="panel-heading">
        <div>
          <p className="eyebrow">Goal</p>
          <h2 id="goal-heading">Agent task</h2>
        </div>
        <Button type="button" variant="ghost" icon={<FileUp size={16} />} onClick={handleLoadFile}>
          Load file
        </Button>
      </div>
      <textarea
        className="goal-textarea"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder="Describe the implementation goal for the selected ACP agent."
        spellCheck={false}
      />
    </section>
  );
}
