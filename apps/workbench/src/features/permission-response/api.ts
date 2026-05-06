import { invokeCommand } from "../../shared/api";

export function respondAgentPermission(permissionId: string, optionId: string) {
  return invokeCommand<void>("respond_agent_permission", { permissionId, optionId });
}
