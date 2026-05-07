import { invoke } from "@tauri-apps/api/core";

export type AgentsRuntimeSnapshot = {
  namespace: string;
  instance_id: string;
  published_at: string;
  state: "starting" | "ready" | "degraded" | "stopped";
  http_base_url: string | null;
  detail: string | null;
};

export function hasTauriHostRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function fetchAgentsRuntimeSnapshot() {
  return invoke<AgentsRuntimeSnapshot>("agents_runtime_snapshot");
}
