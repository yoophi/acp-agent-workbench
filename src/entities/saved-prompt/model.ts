export type SavedPromptScope = "global" | "workspace";
export type SavedPromptRunMode = "insert" | "send" | "enqueue";

export type SavedPrompt = {
  id: string;
  scope: SavedPromptScope;
  workspaceId?: string | null;
  title: string;
  body: string;
  description?: string | null;
  tags: string[];
  runMode: SavedPromptRunMode;
  createdAt: string;
  updatedAt: string;
  lastUsedAt?: string | null;
  useCount: number;
};

export type CreateSavedPromptInput = {
  scope: SavedPromptScope;
  workspaceId?: string | null;
  title: string;
  body: string;
  description?: string | null;
  tags: string[];
  runMode: SavedPromptRunMode;
};

export type UpdateSavedPromptPatch = Partial<
  Pick<CreateSavedPromptInput, "scope" | "workspaceId" | "title" | "body" | "description" | "tags" | "runMode">
>;
