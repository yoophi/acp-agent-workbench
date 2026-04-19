import type { WorkbenchWindowBootstrap, WorkbenchWindowInfo } from "../../entities/workbench-window";
import { invokeCommand } from "../../shared/api";

export function getWindowBootstrap() {
  return invokeCommand<WorkbenchWindowBootstrap>("get_window_bootstrap");
}

export function listWorkbenchWindows() {
  return invokeCommand<WorkbenchWindowInfo[]>("list_workbench_windows");
}

export function openWorkbenchWindow() {
  return invokeCommand<WorkbenchWindowInfo>("open_workbench_window");
}
