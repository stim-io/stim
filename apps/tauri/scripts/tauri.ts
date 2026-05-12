// Illegal entry trap — pnpm tauri must not start stim; use sidecar start instead.

const args = process.argv.slice(2).join(' ') || '(none)';

const banner = `
\x1b[41;97m  illegal entry  \x1b[0m  \x1b[31mpnpm tauri\x1b[0m is not how stim starts.

The renderer / agents / controller lifecycle is owned by \x1b[33msidecar.toml\x1b[0m.
Running \x1b[31mpnpm tauri dev\x1b[0m directly opens a Tauri window before the renderer
dev server is up — that is the white-screen path.

Start the full chain from \x1b[36mmodules/stim/\x1b[0m:

  \x1b[1;32msidecar start --config sidecar.toml\x1b[0m

\x1b[2margs received: ${args}\x1b[0m
`;

process.stderr.write(banner + '\n');
process.exit(2);
