import { invoke } from "@tauri-apps/api/core";

export type ControllerRuntimeSnapshot = {
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

export async function fetchControllerRuntimeSnapshot() {
  return invoke<ControllerRuntimeSnapshot>("controller_runtime_snapshot");
}
