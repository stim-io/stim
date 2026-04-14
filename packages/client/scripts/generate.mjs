import { createClient } from "@hey-api/openapi-ts";

const root = new URL("../", import.meta.url);

await createClient({
  input: new URL("./openapi/stim-server.json", root).href,
  output: new URL("./src/gen", root).pathname,
  plugins: ["@hey-api/client-fetch"]
});
