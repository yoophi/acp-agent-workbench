import { open } from "@tauri-apps/plugin-dialog";
import { FileUp } from "lucide-react";
import { Button, Card, CardHeader, CardTitle, CardTitleBlock, Textarea } from "../../shared/ui";
import { loadGoalFile } from "./api";

type GoalEditorProps = {
  value: string;
  onChange: (value: string) => void;
  onError: (value: string | null) => void;
  readOnly?: boolean;
};

export function GoalEditor({ value, onChange, onError, readOnly = false }: GoalEditorProps) {
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
    <Card as="section" className="flex min-h-0 flex-col" aria-labelledby="goal-heading">
      <CardHeader>
        <CardTitleBlock>
          <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">Goal</p>
          <CardTitle id="goal-heading">Agent task</CardTitle>
        </CardTitleBlock>
        <Button
          type="button"
          variant="ghost"
          icon={<FileUp size={16} />}
          onClick={handleLoadFile}
          disabled={readOnly}
        >
          Load file
        </Button>
      </CardHeader>
      <Textarea
        className="min-h-[200px] flex-1 resize-none rounded-none border-0 p-6 text-base leading-7 shadow-none focus-visible:ring-0"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder="Describe the implementation goal for the selected ACP agent."
        spellCheck={false}
        readOnly={readOnly}
      />
    </Card>
  );
}
