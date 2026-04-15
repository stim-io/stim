import { invoke } from "@tauri-apps/api/core";

export type ControllerRuntimeSnapshot = {
  namespace: string;
  instance_id: string;
  published_at: string;
  state: "starting" | "ready" | "degraded" | "stopped";
  http_base_url: string | null;
  detail: string | null;
};

export async function fetchControllerRuntimeSnapshot() {
  return invoke<ControllerRuntimeSnapshot>("controller_runtime_snapshot");
}
