import { client, configureStimServerClient } from "@stim-io/client";

export function buildStimServerDiscoveryUrl(endpointId: string) {
  configureStimServerClient("http://127.0.0.1:8080");

  return client.buildUrl({
    path: { endpoint_id: endpointId },
    url: "/api/v1/discovery/endpoints/{endpoint_id}"
  });
}
