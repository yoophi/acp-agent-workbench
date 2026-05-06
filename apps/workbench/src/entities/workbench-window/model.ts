export type WorkbenchWindowInfo = {
  label: string;
  isMain: boolean;
  title: string;
};

export type WorkbenchWindowBootstrap = {
  label: string;
  isMain: boolean;
  detachedTab?: unknown;
};

export type WorkbenchWindowCloseRequest = {
  activeRunCount: number;
  lastWindow: boolean;
};
