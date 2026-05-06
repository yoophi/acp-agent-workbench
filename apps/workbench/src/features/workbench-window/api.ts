import type {
  WorkbenchWindowBootstrap,
  WorkbenchWindowCloseRequest,
  WorkbenchWindowInfo,
} from "../../entities/workbench-window";
import { invokeCommand, listenEvent } from "../../shared/api";

export function getWindowBootstrap() {
  return invokeCommand<WorkbenchWindowBootstrap>("get_window_bootstrap");
}

export function listWorkbenchWindows() {
  return invokeCommand<WorkbenchWindowInfo[]>("list_workbench_windows");
}

export function openWorkbenchWindow() {
  return invokeCommand<WorkbenchWindowInfo>("open_workbench_window");
}

export function closeWorkbenchWindow() {
  return invokeCommand<void>("close_workbench_window");
}

export function listenWorkbenchWindowCloseRequests(
  callback: (request: WorkbenchWindowCloseRequest) => void,
) {
  return listenEvent<WorkbenchWindowCloseRequest>("workbench-window-close-requested", callback);
}
