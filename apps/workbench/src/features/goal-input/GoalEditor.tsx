import { open } from "@tauri-apps/plugin-dialog";
import { FileUp } from "lucide-react";
import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardTitleBlock,
  PromptInput,
  PromptInputAction,
  PromptInputActions,
  PromptInputTextarea,
} from "@acp/ui";
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
      </CardHeader>
      <CardContent className="flex min-h-0 flex-1 p-4 pt-4">
        <PromptInput
          className="flex min-h-[220px] flex-1 flex-col"
          disabled={readOnly}
          maxHeight="100%"
          onValueChange={onChange}
          value={value}
        >
          <PromptInputTextarea
            className="min-h-[170px] flex-1 text-base leading-7"
            disableAutosize
            placeholder="Describe the implementation goal for the selected ACP agent."
            readOnly={readOnly}
            spellCheck={false}
          />
          <PromptInputActions className="justify-end border-t pt-3">
            <PromptInputAction tooltip="Load goal from .txt or .md file">
              <Button
                type="button"
                variant="outline"
                size="sm"
                icon={<FileUp size={16} />}
                onClick={handleLoadFile}
                disabled={readOnly}
              >
                Load file
              </Button>
            </PromptInputAction>
          </PromptInputActions>
        </PromptInput>
      </CardContent>
    </Card>
  );
}
