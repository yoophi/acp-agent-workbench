import type { ReactNode } from "react";
import { ArrowLeft, Bell, Database, FolderGit2, Save, Settings, Shield } from "lucide-react";
import { useActiveTabId, useAgentRun } from "../../features/agent-run";
import { Button, Card, CardContent, CardHeader, CardTitle, CardTitleBlock, Input } from "../../shared/ui";
import { SavedPromptsPanel } from "../../widgets/saved-prompts";

type SettingsPageProps = {
  onBack: () => void;
};

export function SettingsPage({ onBack }: SettingsPageProps) {
  const activeTabId = useActiveTabId();
  const state = useAgentRun(activeTabId);

  return (
    <main className="mx-auto flex h-dvh min-h-screen w-full max-w-[1180px] flex-col overflow-hidden p-6 max-lg:h-auto max-lg:min-h-dvh max-lg:overflow-visible max-lg:p-4 max-sm:p-3">
      <header className="mb-6 flex items-center justify-between gap-4 max-sm:flex-col max-sm:items-stretch">
        <div className="flex min-w-0 items-center gap-3">
          <Button variant="outline" size="icon" aria-label="Back to workbench" onClick={onBack}>
            <ArrowLeft />
          </Button>
          <div className="min-w-0">
            <p className="mb-1.5 text-xs font-medium uppercase tracking-wide text-muted-foreground">
              ACP Agent Workbench
            </p>
            <h1 className="m-0 flex items-center gap-2 text-2xl font-semibold leading-tight text-foreground">
              <Settings className="size-5" />
              Settings
            </h1>
          </div>
        </div>
        <Button icon={<Save />} onClick={onBack}>Done</Button>
      </header>

      <div className="grid min-h-0 flex-1 grid-cols-[260px_minmax(0,1fr)] gap-4 max-lg:grid-cols-1">
        <aside className="rounded-lg border bg-card p-2 text-sm shadow-card max-lg:flex max-lg:overflow-x-auto">
          <SettingsNavItem icon={<FolderGit2 className="size-4" />} label="Work Contexts" active />
          <SettingsNavItem icon={<Database className="size-4" />} label="Storage" />
          <SettingsNavItem icon={<Shield className="size-4" />} label="Permissions" />
          <SettingsNavItem icon={<Bell className="size-4" />} label="Notifications" />
        </aside>

        <section className="min-h-0 overflow-y-auto rounded-lg border bg-card shadow-card">
          <div className="border-b px-6 py-5">
            <h2 className="m-0 text-lg font-semibold">Work Contexts</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Configure agent execution defaults and reusable prompts for the active workbench tab.
            </p>
          </div>

          <div className="grid gap-4 p-6">
            <Card>
              <CardHeader>
                <CardTitleBlock>
                  <CardTitle>Coding agent commands</CardTitle>
                  <p className="mt-1 text-sm text-muted-foreground">
                    Configure a custom ACP launcher command for each coding agent.
                  </p>
                </CardTitleBlock>
              </CardHeader>
              <CardContent className="grid gap-4">
                {state.agents.map((agent) => (
                  <label key={agent.id} className="grid gap-1.5">
                    <span className="flex items-center justify-between gap-3 text-sm font-medium">
                      <span>{agent.label}</span>
                      {agent.id === state.selectedAgentId ? (
                        <span className="rounded-md bg-accent px-2 py-0.5 text-xs text-accent-foreground">
                          selected
                        </span>
                      ) : null}
                    </span>
                    <Input
                      value={state.agentCommandOverrides[agent.id] ?? ""}
                      onChange={(event) => state.setAgentCommandOverride(agent.id, event.target.value)}
                      placeholder={agent.command}
                      disabled={state.sessionActive}
                    />
                  </label>
                ))}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitleBlock>
                  <CardTitle>Runtime limits</CardTitle>
                </CardTitleBlock>
              </CardHeader>
              <CardContent className="grid gap-4">
                <label className="grid gap-1.5">
                  <span className="text-sm font-medium">Stdio buffer</span>
                  <Input
                    type="number"
                    min={1}
                    max={512}
                    value={state.stdioBufferLimitMb}
                    disabled={state.sessionActive}
                    onChange={(event) => state.setStdioBufferLimitMb(Number(event.target.value))}
                  />
                </label>
                <label className="grid gap-1.5">
                  <span className="text-sm font-medium">Idle timeout (sec, 0 = off)</span>
                  <Input
                    type="number"
                    min={0}
                    max={3600}
                    value={state.idleTimeoutSec}
                    disabled={state.sessionActive}
                    onChange={(event) => state.setIdleTimeoutSec(Math.max(0, Number(event.target.value) || 0))}
                  />
                </label>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitleBlock>
                  <CardTitle>Agent behavior</CardTitle>
                </CardTitleBlock>
              </CardHeader>
              <CardContent className="grid gap-3">
                <SettingCheck
                  label="Auto-select allow permission"
                  checked={state.autoAllow}
                  onChange={state.setAutoAllow}
                />
                <SettingCheck
                  label="Stop on permission request during Ralph loop"
                  checked={state.ralphLoop.stopOnPermission}
                  onChange={(checked) => state.setRalphLoop({ ...state.ralphLoop, stopOnPermission: checked })}
                  disabled={!state.ralphLoop.enabled || state.sessionActive}
                />
                <SettingCheck
                  label="Stop Ralph loop on error"
                  checked={state.ralphLoop.stopOnError}
                  onChange={(checked) => state.setRalphLoop({ ...state.ralphLoop, stopOnError: checked })}
                  disabled={!state.ralphLoop.enabled || state.sessionActive}
                />
              </CardContent>
            </Card>

            <SavedPromptsPanel
              workspaceId={state.workspaceId}
              sessionActive={state.sessionActive}
              onApply={state.applySavedPrompt}
              onError={state.setError}
            />
          </div>
        </section>
      </div>
    </main>
  );
}

function SettingsNavItem({
  icon,
  label,
  active = false,
}: {
  icon: ReactNode;
  label: string;
  active?: boolean;
}) {
  return (
    <button
      type="button"
      className={[
        "flex w-full items-center gap-2 rounded-md px-3 py-2 text-left font-medium transition-colors max-lg:w-auto max-lg:shrink-0",
        active ? "bg-accent text-accent-foreground" : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
      ].join(" ")}
    >
      {icon}
      <span>{label}</span>
    </button>
  );
}

function SettingCheck({
  label,
  checked,
  onChange,
  disabled = false,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <label className="flex items-center justify-between gap-4 rounded-md border px-3 py-2.5">
      <span className="text-sm font-medium">{label}</span>
      <input
        type="checkbox"
        checked={checked}
        disabled={disabled}
        className="size-4 accent-primary disabled:cursor-not-allowed disabled:opacity-50"
        onChange={(event) => onChange(event.target.checked)}
      />
    </label>
  );
}
