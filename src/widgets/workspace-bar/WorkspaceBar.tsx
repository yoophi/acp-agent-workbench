import { FolderGit2, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import {
  listWorkspaceCheckouts,
  listWorkspaces,
  registerWorkspaceFromPath,
  setTabWorkdir,
  setTabWorkspace,
  setWorkbenchWorkspaces,
  setWorkspaceCheckouts,
  setWorkspaceError,
  upsertWorkspaceRegistration,
  useWorkspaceState,
} from "../../features/agent-run";
import { Badge, Button, Input, NativeSelect } from "../../shared/ui";

type WorkspaceBarProps = {
  tabId: string;
  disabled?: boolean;
};

export function WorkspaceBar({ tabId, disabled = false }: WorkspaceBarProps) {
  const state = useWorkspaceState(tabId);
  const [repoPath, setRepoPath] = useState("");
  const [loading, setLoading] = useState(false);

  const selectedWorkspaceId = state.workspaceId ?? "";
  const selectedCheckoutId = state.checkoutId ?? "";

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const workspaces = await listWorkspaces();
      setWorkbenchWorkspaces(workspaces);
      await Promise.all(
        workspaces.map(async (workspace) => {
          const checkouts = await listWorkspaceCheckouts(workspace.id);
          setWorkspaceCheckouts(workspace.id, checkouts);
        }),
      );
      setWorkspaceError(null);
    } catch (err) {
      setWorkspaceError(String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const checkoutOptions = state.checkouts;
  const workdirPlaceholder = useMemo(
    () => state.selectedCheckout?.path ?? "Select a workspace checkout",
    [state.selectedCheckout?.path],
  );

  const handleRegister = useCallback(async () => {
    const trimmed = repoPath.trim();
    if (!trimmed) return;
    setLoading(true);
    try {
      const registered = await registerWorkspaceFromPath(trimmed);
      upsertWorkspaceRegistration(registered.workspace, registered.checkout);
      setTabWorkspace(tabId, registered.workspace.id, registered.checkout.id);
      setTabWorkdir(tabId, registered.checkout.path);
      setRepoPath("");
      setWorkspaceError(null);
    } catch (err) {
      setWorkspaceError(String(err));
    } finally {
      setLoading(false);
    }
  }, [repoPath, tabId]);

  const handleWorkspaceChange = useCallback(
    async (workspaceId: string) => {
      if (!workspaceId) {
        setTabWorkspace(tabId, null, null);
        return;
      }
      try {
        const checkouts = await listWorkspaceCheckouts(workspaceId);
        setWorkspaceCheckouts(workspaceId, checkouts);
        const checkout = checkouts.find((entry) => entry.isDefault) ?? checkouts[0];
        setTabWorkspace(tabId, workspaceId, checkout?.id ?? null);
      } catch (err) {
        setWorkspaceError(String(err));
      }
    },
    [tabId],
  );

  const handleCheckoutChange = useCallback(
    (checkoutId: string) => {
      const checkout = checkoutOptions.find((entry) => entry.id === checkoutId);
      setTabWorkspace(tabId, selectedWorkspaceId || null, checkoutId || null);
      if (checkout) setTabWorkdir(tabId, checkout.path);
    },
    [checkoutOptions, selectedWorkspaceId, tabId],
  );

  return (
    <section className="mb-4 grid gap-3 rounded-lg border bg-card/80 p-3 shadow-sm">
      <div className="grid grid-cols-[minmax(180px,0.8fr)_minmax(180px,0.8fr)_minmax(240px,1.2fr)] gap-3 max-lg:grid-cols-1">
        <label className="grid gap-1.5">
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Workspace
          </span>
          <NativeSelect
            value={selectedWorkspaceId}
            disabled={disabled || loading}
            onChange={(event) => void handleWorkspaceChange(event.target.value)}
          >
            <option value="">Legacy working directory</option>
            {state.workspaces.map((workspace) => (
              <option key={workspace.id} value={workspace.id}>
                {workspace.name} ({workspace.origin.owner}/{workspace.origin.repo})
              </option>
            ))}
          </NativeSelect>
        </label>

        <label className="grid gap-1.5">
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Checkout
          </span>
          <NativeSelect
            value={selectedCheckoutId}
            disabled={disabled || loading || !selectedWorkspaceId}
            onChange={(event) => handleCheckoutChange(event.target.value)}
          >
            <option value="">Default checkout</option>
            {checkoutOptions.map((checkout) => (
              <option key={checkout.id} value={checkout.id}>
                {checkout.branch || "detached"} · {checkout.path}
              </option>
            ))}
          </NativeSelect>
        </label>

        <label className="grid gap-1.5">
          <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
            Working Directory
          </span>
          <Input
            value={state.workdir}
            placeholder={workdirPlaceholder}
            disabled={disabled}
            onChange={(event) => setTabWorkdir(tabId, event.target.value)}
          />
        </label>
      </div>

      <div className="grid grid-cols-[minmax(260px,1fr)_auto_auto] items-center gap-2 max-lg:grid-cols-1">
        <Input
          value={repoPath}
          disabled={disabled || loading}
          placeholder="Register local GitHub repo path"
          onChange={(event) => setRepoPath(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") void handleRegister();
          }}
        />
        <Button
          type="button"
          variant="secondary"
          icon={<FolderGit2 size={16} />}
          disabled={disabled || loading || !repoPath.trim()}
          onClick={() => void handleRegister()}
        >
          Register
        </Button>
        <Button
          type="button"
          variant="outline"
          icon={<RefreshCw size={16} />}
          disabled={loading}
          onClick={() => void load()}
        >
          Refresh
        </Button>
      </div>

      <div className="flex min-h-6 flex-wrap items-center gap-2 text-xs text-muted-foreground">
        {state.selectedWorkspace ? (
          <span>{state.selectedWorkspace.origin.canonicalUrl}</span>
        ) : (
          <span>Runs without a selected workspace use the working directory directly.</span>
        )}
        {state.activeSameWorkdirCount > 0 ? (
          <Badge variant="secondary">
            shared directory +{state.activeSameWorkdirCount}
          </Badge>
        ) : null}
        {state.workspaceError ? (
          <span className="font-medium text-destructive">{state.workspaceError}</span>
        ) : null}
      </div>
    </section>
  );
}
