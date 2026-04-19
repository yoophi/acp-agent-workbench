import { Plus, Save, Trash2, Wand2 } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { SavedPrompt, SavedPromptRunMode, SavedPromptScope } from "../../entities/saved-prompt";
import {
  createSavedPrompt,
  deleteSavedPrompt,
  listSavedPrompts,
  recordSavedPromptUsed,
  updateSavedPrompt,
} from "../../features/agent-run";
import { Button, Card, CardContent, CardHeader, CardTitle, CardTitleBlock, Input, NativeSelect, Textarea } from "../../shared/ui";

type SavedPromptsPanelProps = {
  workspaceId?: string | null;
  sessionActive: boolean;
  onApply: (body: string, runMode: SavedPromptRunMode) => void;
  onError: (error: string | null) => void;
};

type FormState = {
  id: string | null;
  scope: SavedPromptScope;
  title: string;
  body: string;
  tags: string;
  runMode: SavedPromptRunMode;
};

const emptyForm: FormState = {
  id: null,
  scope: "global",
  title: "",
  body: "",
  tags: "",
  runMode: "enqueue",
};

export function SavedPromptsPanel({ workspaceId, sessionActive, onApply, onError }: SavedPromptsPanelProps) {
  const [prompts, setPrompts] = useState<SavedPrompt[]>([]);
  const [loading, setLoading] = useState(false);
  const [form, setForm] = useState<FormState>(emptyForm);
  const [expanded, setExpanded] = useState(false);

  async function load() {
    setLoading(true);
    try {
      setPrompts(await listSavedPrompts(workspaceId));
      onError(null);
    } catch (err) {
      onError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
  }, [workspaceId]);

  const quickPrompts = useMemo(() => prompts.slice(0, 4), [prompts]);
  const canSave = form.title.trim().length > 0 && form.body.trim().length > 0;

  async function applyPrompt(prompt: SavedPrompt) {
    try {
      onApply(prompt.body, prompt.runMode);
      await recordSavedPromptUsed(prompt.id);
      await load();
    } catch (err) {
      onError(String(err));
    }
  }

  async function savePrompt() {
    if (!canSave) return;
    const payload = {
      scope: form.scope,
      workspaceId: form.scope === "workspace" ? workspaceId : null,
      title: form.title,
      body: form.body,
      tags: form.tags.split(",").map((tag) => tag.trim()).filter(Boolean),
      runMode: form.runMode,
    };
    try {
      if (form.id) {
        await updateSavedPrompt(form.id, payload);
      } else {
        await createSavedPrompt(payload);
      }
      setForm(emptyForm);
      setExpanded(false);
      await load();
      onError(null);
    } catch (err) {
      onError(String(err));
    }
  }

  async function removePrompt(id: string) {
    try {
      await deleteSavedPrompt(id);
      if (form.id === id) setForm(emptyForm);
      await load();
    } catch (err) {
      onError(String(err));
    }
  }

  function editPrompt(prompt: SavedPrompt) {
    setForm({
      id: prompt.id,
      scope: prompt.scope,
      title: prompt.title,
      body: prompt.body,
      tags: prompt.tags.join(", "),
      runMode: prompt.runMode,
    });
    setExpanded(true);
  }

  return (
    <Card as="section" aria-labelledby="saved-prompts-heading">
      <CardHeader>
        <CardTitleBlock>
          <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">Quick prompts</p>
          <CardTitle id="saved-prompts-heading">Saved prompts</CardTitle>
        </CardTitleBlock>
        <Button
          type="button"
          variant="outline"
          size="sm"
          icon={<Plus size={16} />}
          onClick={() => setExpanded((value) => !value)}
        >
          {expanded ? "Close" : "New"}
        </Button>
      </CardHeader>
      <CardContent className="grid gap-3 pt-6">
        <div className="flex flex-wrap gap-2">
          {quickPrompts.length > 0 ? (
            quickPrompts.map((prompt) => (
              <Button
                key={prompt.id}
                type="button"
                variant="secondary"
                size="sm"
                icon={<Wand2 size={16} />}
                disabled={loading}
                onClick={() => void applyPrompt(prompt)}
                title={prompt.body}
              >
                {prompt.title}
              </Button>
            ))
          ) : (
            <span className="text-sm text-muted-foreground">No saved prompts yet.</span>
          )}
        </div>

        {expanded ? (
          <div className="grid gap-2 rounded-md border border-border p-3">
            <div className="grid grid-cols-[1fr_150px_150px] gap-2 max-lg:grid-cols-1">
              <Input
                value={form.title}
                placeholder="Prompt title"
                onChange={(event) => setForm((current) => ({ ...current, title: event.target.value }))}
              />
              <NativeSelect
                value={form.scope}
                disabled={!workspaceId}
                onChange={(event) =>
                  setForm((current) => ({ ...current, scope: event.target.value as SavedPromptScope }))
                }
              >
                <option value="global">Global</option>
                <option value="workspace">Workspace</option>
              </NativeSelect>
              <NativeSelect
                value={form.runMode}
                onChange={(event) =>
                  setForm((current) => ({ ...current, runMode: event.target.value as SavedPromptRunMode }))
                }
              >
                <option value="insert">Insert</option>
                <option value="send">Send</option>
                <option value="enqueue">Enqueue</option>
              </NativeSelect>
            </div>
            <Textarea
              className="min-h-[90px] resize-y"
              value={form.body}
              placeholder={sessionActive ? "Follow-up prompt body" : "Goal prompt body"}
              onChange={(event) => setForm((current) => ({ ...current, body: event.target.value }))}
            />
            <Input
              value={form.tags}
              placeholder="Tags, comma separated"
              onChange={(event) => setForm((current) => ({ ...current, tags: event.target.value }))}
            />
            <div className="flex flex-wrap justify-between gap-2">
              <div className="flex flex-wrap gap-2">
                {prompts.map((prompt) => (
                  <Button key={prompt.id} type="button" variant="ghost" size="sm" onClick={() => editPrompt(prompt)}>
                    {prompt.title}
                  </Button>
                ))}
              </div>
              <div className="flex gap-2">
                {form.id ? (
                  <Button
                    type="button"
                    variant="destructive"
                    size="sm"
                    icon={<Trash2 size={16} />}
                    onClick={() => void removePrompt(form.id!)}
                  >
                    Delete
                  </Button>
                ) : null}
                <Button
                  type="button"
                  variant="primary"
                  size="sm"
                  icon={<Save size={16} />}
                  disabled={!canSave || loading}
                  onClick={() => void savePrompt()}
                >
                  Save
                </Button>
              </div>
            </div>
          </div>
        ) : null}
      </CardContent>
    </Card>
  );
}
