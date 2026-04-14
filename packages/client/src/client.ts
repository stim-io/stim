import { client } from "./gen/client.gen";

export function configureStimServerClient(baseUrl: string) {
  client.setConfig({ baseUrl });
}

export { client };
