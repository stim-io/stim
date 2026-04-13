type RendererProbeRequest = {
  request_id: string;
  requested_at: string;
  probe: {
    probe: "landing-basics";
  };
};

type RendererProbeResponse = {
  request_id: string;
  requested_at: string;
  result:
    | {
        kind: "success";
        snapshot: {
          inspected_at: string;
          probe: {
            kind: "landing-basics";
            document_ready_state: string;
            document_title: string;
            landing_shell_present: boolean;
            landing_card_present: boolean;
            landing_title_text: string | null;
            primary_action_label: string | null;
          };
        };
      }
    | {
        kind: "failure";
        reason: "probe-failed";
      };
};

const REQUEST_EVENT = "stim://inspection/renderer-probe-request";
const RESPONSE_EVENT = "stim://inspection/renderer-probe-response";

function timestampNow(): string {
  const now = Date.now();
  return `${Math.floor(now / 1000)}-${String(now % 1000).padStart(3, "0")}`;
}

function hasTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function textContentFor(selector: string): string | null {
  return document.querySelector(selector)?.textContent?.trim() || null;
}

function primaryActionLabel(): string | null {
  return textContentFor('[data-probe="landing-actions"] button');
}

function buildLandingBasicsResponse(request: RendererProbeRequest): RendererProbeResponse {
  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "success",
      snapshot: {
        inspected_at: timestampNow(),
        probe: {
          kind: "landing-basics",
          document_ready_state: document.readyState,
          document_title: document.title,
          landing_shell_present: Boolean(document.querySelector('[data-probe="landing-shell"]')),
          landing_card_present: Boolean(document.querySelector('[data-probe="landing-card"]')),
          landing_title_text: textContentFor('[data-probe="landing-title"]'),
          primary_action_label: primaryActionLabel(),
        },
      },
    },
  };
}

function buildFailureResponse(request: RendererProbeRequest): RendererProbeResponse {
  return {
    request_id: request.request_id,
    requested_at: request.requested_at,
    result: {
      kind: "failure",
      reason: "probe-failed",
    },
  };
}

async function handleProbeRequest(request: RendererProbeRequest) {
  const { emit } = await import("@tauri-apps/api/event");

  try {
    const response = request.probe.probe === "landing-basics"
      ? buildLandingBasicsResponse(request)
      : buildFailureResponse(request);

    await emit(RESPONSE_EVENT, response);
  } catch {
    await emit(RESPONSE_EVENT, buildFailureResponse(request));
  }
}

export async function setupInspectionProbes() {
  if (!hasTauriRuntime()) {
    return;
  }

  const { listen } = await import("@tauri-apps/api/event");

  await listen<RendererProbeRequest>(REQUEST_EVENT, async (event) => {
    if (!event.payload) {
      return;
    }

    await handleProbeRequest(event.payload);
  });
}
