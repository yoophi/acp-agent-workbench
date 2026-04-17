import { invokeCommand } from "../../shared/api";

export function loadGoalFile(path: string) {
  return invokeCommand<string>("load_goal_file", { path });
}
