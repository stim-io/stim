use std::{
    env,
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    time::Duration,
};

use serde::Serialize;

use crate::{
    clock::timestamp_now,
    runtime_control::{current_namespace, stamped_processes_for_namespace},
};

const DEFAULT_STIM_SERVER_BASE_URL: &str = "http://127.0.0.1:18083";
const DEFAULT_SANTI_BASE_URL: &str = "http://127.0.0.1:18081";
const DEFAULT_SANTI_LINK_BASE_URL: &str = "http://127.0.0.1:18082";

const STANDALONE_COMPOSE_HINT: &str = "docker compose up -d --build stim-server santi-link santi";

pub(crate) fn detect() -> Result<(), String> {
    let report = DetectReport::collect();
    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to serialize detect report: {error}"))?;

    println!("{output}");
    Ok(())
}

#[derive(Serialize)]
struct DetectReport {
    command: &'static str,
    mode: &'static str,
    namespace: String,
    checked_at: String,
    root_workspace: RootWorkspaceProbe,
    files: FileProbes,
    app_loop: AppLoopProbe,
    services: Vec<ServiceProbe>,
    summary: DetectSummary,
    hints: Vec<String>,
}

impl DetectReport {
    fn collect() -> Self {
        let stim_root = stim_platform::paths::workspace_root();
        let root_workspace = RootWorkspaceProbe::collect(&stim_root);
        let files = FileProbes::collect(root_workspace.path.as_deref());
        let namespace = current_namespace();
        let app_loop = AppLoopProbe::collect(&namespace);
        let services = vec![
            ServiceProbe::check(
                "stim-server",
                Some("STIM_SERVER_BASE_URL"),
                DEFAULT_STIM_SERVER_BASE_URL,
                "/api/v1/health",
            ),
            ServiceProbe::check(
                "santi",
                Some("SANTI_BASE_URL"),
                DEFAULT_SANTI_BASE_URL,
                "/api/v1/health",
            ),
            ServiceProbe::check(
                "santi-link",
                None,
                DEFAULT_SANTI_LINK_BASE_URL,
                "/openai/v1/health",
            ),
        ];
        let summary = summarize(&files, &services);
        let hints = operation_hints(&root_workspace, &files, &services, &summary, &app_loop);

        Self {
            command: "stim-dev detect",
            mode: "standalone-prerequisites",
            namespace,
            checked_at: timestamp_now(),
            root_workspace,
            files,
            app_loop,
            services,
            summary,
            hints,
        }
    }
}

#[derive(Serialize)]
struct RootWorkspaceProbe {
    state: &'static str,
    path: Option<String>,
    compose_file: PathProbe,
}

impl RootWorkspaceProbe {
    fn collect(stim_root: &Path) -> Self {
        let Some(root) = attached_root_candidate(stim_root) else {
            return Self {
                state: "not-attached",
                path: None,
                compose_file: PathProbe::missing(
                    "root docker-compose.yml",
                    None,
                    "stim workspace is not attached under a root modules/ directory",
                ),
            };
        };

        let compose_file =
            PathProbe::file("root docker-compose.yml", root.join("docker-compose.yml"));
        let state = if compose_file.state == "present" {
            "attached"
        } else {
            "compose-file-missing"
        };

        Self {
            state,
            path: Some(root.to_string_lossy().to_string()),
            compose_file,
        }
    }
}

#[derive(Serialize)]
struct FileProbes {
    santi_link_auth: PathProbe,
}

impl FileProbes {
    fn collect(root_workspace: Option<&str>) -> Self {
        let santi_link_auth = root_workspace
            .map(|root| PathBuf::from(root).join("modules/santi-link/auth.json"))
            .map(|path| PathProbe::file("santi-link auth.json", path))
            .unwrap_or_else(|| {
                PathProbe::missing(
                    "santi-link auth.json",
                    None,
                    "root workspace was not detected",
                )
            });

        Self { santi_link_auth }
    }
}

#[derive(Serialize)]
struct PathProbe {
    label: &'static str,
    path: Option<String>,
    state: &'static str,
    detail: String,
}

impl PathProbe {
    fn file(label: &'static str, path: PathBuf) -> Self {
        if path.is_file() {
            Self {
                label,
                path: Some(path.to_string_lossy().to_string()),
                state: "present",
                detail: "file exists".into(),
            }
        } else {
            Self::missing(label, Some(path), "file is missing")
        }
    }

    fn missing(label: &'static str, path: Option<PathBuf>, detail: &str) -> Self {
        Self {
            label,
            path: path.map(|path| path.to_string_lossy().to_string()),
            state: "missing",
            detail: detail.into(),
        }
    }

    fn directory(label: &'static str, path: PathBuf) -> Self {
        if path.is_dir() {
            Self {
                label,
                path: Some(path.to_string_lossy().to_string()),
                state: "present",
                detail: "directory exists".into(),
            }
        } else {
            Self::missing(label, Some(path), "directory is missing")
        }
    }
}

#[derive(Serialize)]
struct AppLoopProbe {
    state: &'static str,
    detail: String,
    stamped_process_count: usize,
    stamped_processes: Vec<ProcessProbe>,
    residue: AppLoopResidueProbe,
}

impl AppLoopProbe {
    fn collect(namespace: &str) -> Self {
        let residue = AppLoopResidueProbe::collect(namespace);
        let processes = match stamped_processes_for_namespace(namespace) {
            Ok(processes) => processes,
            Err(error) => {
                return Self {
                    state: "unknown",
                    detail: format!("failed to inspect stamped processes: {error}"),
                    stamped_process_count: 0,
                    stamped_processes: Vec::new(),
                    residue,
                };
            }
        };
        let stamped_processes = processes
            .into_iter()
            .map(|process| ProcessProbe {
                pid: process.pid,
                ppid: process.ppid,
                command: process.command,
            })
            .collect::<Vec<_>>();
        let stamped_process_count = stamped_processes.len();
        let state = if stamped_process_count > 0 {
            "running"
        } else if residue.is_present() {
            "stopped-with-residue"
        } else {
            "stopped-clean"
        };
        let detail = match state {
            "running" => format!("{stamped_process_count} stamped app-loop processes found"),
            "stopped-with-residue" => {
                "no stamped app-loop processes found, but bridge/log/lock residue exists".into()
            }
            _ => "no stamped app-loop processes or residue found".into(),
        };

        Self {
            state,
            detail,
            stamped_process_count,
            stamped_processes,
            residue,
        }
    }

    fn is_running(&self) -> bool {
        self.state == "running"
    }

    fn has_residue_without_processes(&self) -> bool {
        self.state == "stopped-with-residue"
    }
}

#[derive(Serialize)]
struct ProcessProbe {
    pid: u32,
    ppid: u32,
    command: String,
}

#[derive(Serialize)]
struct AppLoopResidueProbe {
    bridges: PathProbe,
    logs: PathProbe,
    locks: PathProbe,
}

impl AppLoopResidueProbe {
    fn collect(namespace: &str) -> Self {
        let layout = stim_sidecar::layout::SidecarLayout::new(
            stim_platform::paths::dev_root(),
            Some(namespace),
        );

        Self {
            bridges: PathProbe::directory("app-loop bridges", layout.bridges_root),
            logs: PathProbe::directory("app-loop logs", layout.logs_root),
            locks: PathProbe::directory("app-loop locks", layout.locks_root),
        }
    }

    fn is_present(&self) -> bool {
        self.bridges.state == "present"
            || self.logs.state == "present"
            || self.locks.state == "present"
    }
}

#[derive(Serialize)]
struct ServiceProbe {
    name: &'static str,
    source: &'static str,
    env_var: Option<&'static str>,
    base_url: String,
    health_path: &'static str,
    state: &'static str,
    detail: String,
}

impl ServiceProbe {
    fn check(
        name: &'static str,
        env_var: Option<&'static str>,
        default_base_url: &'static str,
        health_path: &'static str,
    ) -> Self {
        let env_base_url = env_var
            .and_then(|key| env::var(key).ok())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let source = if env_base_url.is_some() {
            "env-override"
        } else {
            "compose-default"
        };
        let base_url = env_base_url.unwrap_or_else(|| default_base_url.to_string());

        match http_get_status(&base_url, health_path, Duration::from_millis(700)) {
            Ok(status) if (200..300).contains(&status) => Self {
                name,
                source,
                env_var,
                base_url,
                health_path,
                state: "ready",
                detail: format!("health returned HTTP {status}"),
            },
            Ok(status) => Self {
                name,
                source,
                env_var,
                base_url,
                health_path,
                state: "unhealthy",
                detail: format!("health returned HTTP {status}"),
            },
            Err(error) => Self {
                name,
                source,
                env_var,
                base_url,
                health_path,
                state: "unavailable",
                detail: error,
            },
        }
    }

    fn is_ready(&self) -> bool {
        self.state == "ready"
    }

    fn uses_compose_default(&self) -> bool {
        self.source == "compose-default"
    }
}

#[derive(Serialize)]
struct DetectSummary {
    state: &'static str,
    ready: bool,
    needs_action: Vec<String>,
}

fn summarize(files: &FileProbes, services: &[ServiceProbe]) -> DetectSummary {
    let mut needs_action = Vec::new();

    if files.santi_link_auth.state != "present" {
        needs_action.push("santi-link auth.json missing".into());
    }

    needs_action.extend(
        services
            .iter()
            .filter(|service| !service.is_ready())
            .map(|service| format!("{} {}", service.name, service.state)),
    );

    let ready = needs_action.is_empty();
    let state = if ready { "ready" } else { "needs-action" };

    DetectSummary {
        state,
        ready,
        needs_action,
    }
}

fn operation_hints(
    root_workspace: &RootWorkspaceProbe,
    files: &FileProbes,
    services: &[ServiceProbe],
    summary: &DetectSummary,
    app_loop: &AppLoopProbe,
) -> Vec<String> {
    let mut hints = vec![
        "stim-dev detect is read-only; it reports prerequisites and suggested next commands but does not start or stop Docker or sidecars.".into(),
    ];

    if services
        .iter()
        .any(|service| !service.is_ready() && service.uses_compose_default())
    {
        hints.push(match root_workspace.path.as_deref() {
            Some(root) if root_workspace.compose_file.state == "present" => format!(
                "Root compose prerequisites are not ready. From {root}, run: {STANDALONE_COMPOSE_HINT}"
            ),
            _ => "Root compose prerequisites are not ready and no attached root docker-compose.yml was detected. Attach under the root workspace or set STIM_SERVER_BASE_URL and SANTI_BASE_URL to reachable services.".into(),
        });
    }

    if files.santi_link_auth.state != "present" {
        hints.push("Provider-backed standalone smoke needs modules/santi-link/auth.json before santi-link can serve real upstream requests.".into());
    }

    if services
        .iter()
        .any(|service| !service.is_ready() && service.source == "env-override")
    {
        hints.push("One or more environment override endpoints are unreachable; fix the env var target or unset it to use the root compose default.".into());
    }

    if app_loop.has_residue_without_processes() {
        hints.push("App-loop bridge/log/lock residue exists without stamped app-loop processes. If a previous restart timed out or you want a clean smoke run, run: stim-dev reset".into());
    }

    if summary.ready {
        if app_loop.is_running() {
            hints.push("Standalone prerequisites are reachable and stamped app-loop processes exist. Next inspection step: stim-dev status".into());
        } else if app_loop.has_residue_without_processes() {
            hints.push("Standalone prerequisites are reachable. Clean app-loop step: stim-dev reset, then stim-dev restart".into());
        } else {
            hints.push(
                "Standalone prerequisites are reachable. Next app-loop step: stim-dev restart"
                    .into(),
            );
        }
    } else {
        hints.push("After applying the suggested prerequisite fix, rerun: stim-dev detect".into());
    }

    hints
}

fn attached_root_candidate(stim_root: &Path) -> Option<PathBuf> {
    let modules_dir = stim_root.parent()?;
    if modules_dir.file_name()? != "modules" {
        return None;
    }

    modules_dir.parent().map(Path::to_path_buf)
}

struct HttpEndpoint {
    host: String,
    port: u16,
    host_header: String,
}

fn http_get_status(base_url: &str, path: &str, timeout: Duration) -> Result<u16, String> {
    let endpoint = parse_http_base_url(base_url)?;
    let address = format!("{}:{}", endpoint.host, endpoint.port);
    let socket_address = address
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve {address}: {error}"))?
        .next()
        .ok_or_else(|| format!("failed to resolve {address}: no socket addresses"))?;
    let mut stream = TcpStream::connect_timeout(&socket_address, timeout)
        .map_err(|error| format!("failed to connect to {address}: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("failed to set read timeout for {address}: {error}"))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|error| format!("failed to set write timeout for {address}: {error}"))?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {}\r\nUser-Agent: stim-dev-detect\r\nConnection: close\r\n\r\n",
        endpoint.host_header
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write health request to {address}: {error}"))?;

    let mut buffer = [0; 512];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| format!("failed to read health response from {address}: {error}"))?;
    if read == 0 {
        return Err(format!("health response from {address} was empty"));
    }

    let response = String::from_utf8_lossy(&buffer[..read]);
    let status_line = response
        .lines()
        .next()
        .ok_or_else(|| format!("health response from {address} had no status line"))?;
    status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("health response from {address} had malformed status line"))?
        .parse::<u16>()
        .map_err(|error| format!("health response from {address} had invalid status code: {error}"))
}

fn parse_http_base_url(base_url: &str) -> Result<HttpEndpoint, String> {
    let authority_and_path = base_url.strip_prefix("http://").ok_or_else(|| {
        format!("unsupported health URL scheme for {base_url}; only http:// is supported")
    })?;
    let authority = authority_and_path
        .split('/')
        .next()
        .filter(|authority| !authority.is_empty())
        .ok_or_else(|| format!("missing host in {base_url}"))?;

    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            let port = port
                .parse::<u16>()
                .map_err(|error| format!("invalid port in {base_url}: {error}"))?;
            (host.to_string(), port)
        }
        None => (authority.to_string(), 80),
        Some(_) => return Err(format!("missing host in {base_url}")),
    };

    let host_header = if port == 80 {
        host.clone()
    } else {
        format!("{host}:{port}")
    };

    Ok(HttpEndpoint {
        host,
        port,
        host_header,
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        attached_root_candidate, operation_hints, parse_http_base_url, summarize, AppLoopProbe,
        AppLoopResidueProbe, DetectSummary, FileProbes, PathProbe, RootWorkspaceProbe,
        ServiceProbe,
    };

    #[test]
    fn detects_attached_root_from_modules_stim_path() {
        let root = attached_root_candidate(Path::new("/workspace/modules/stim")).unwrap();

        assert_eq!(root, Path::new("/workspace"));
        assert!(attached_root_candidate(Path::new("/workspace/stim")).is_none());
    }

    #[test]
    fn parses_plain_http_base_urls_for_local_health_checks() {
        let endpoint = parse_http_base_url("http://127.0.0.1:18081").unwrap();

        assert_eq!(endpoint.host, "127.0.0.1");
        assert_eq!(endpoint.port, 18081);
        assert_eq!(endpoint.host_header, "127.0.0.1:18081");

        let endpoint = parse_http_base_url("http://localhost").unwrap();
        assert_eq!(endpoint.port, 80);
    }

    #[test]
    fn hints_explain_compose_prerequisite_without_owning_lifecycle() {
        let root_workspace = RootWorkspaceProbe {
            state: "attached",
            path: Some("/workspace".into()),
            compose_file: PathProbe {
                label: "root docker-compose.yml",
                path: Some("/workspace/docker-compose.yml".into()),
                state: "present",
                detail: "file exists".into(),
            },
        };
        let files = FileProbes {
            santi_link_auth: PathProbe {
                label: "santi-link auth.json",
                path: Some("/workspace/modules/santi-link/auth.json".into()),
                state: "present",
                detail: "file exists".into(),
            },
        };
        let services = vec![ServiceProbe {
            name: "santi",
            source: "compose-default",
            env_var: Some("SANTI_BASE_URL"),
            base_url: "http://127.0.0.1:18081".into(),
            health_path: "/api/v1/health",
            state: "unavailable",
            detail: "connection refused".into(),
        }];
        let summary = summarize(&files, &services);
        let app_loop = AppLoopProbe {
            state: "stopped-clean",
            detail: "no stamped app-loop processes or residue found".into(),
            stamped_process_count: 0,
            stamped_processes: Vec::new(),
            residue: missing_residue(),
        };

        let hints =
            operation_hints(&root_workspace, &files, &services, &summary, &app_loop).join("\n");

        assert!(hints.contains("docker compose up -d --build stim-server santi-link santi"));
        assert!(hints.contains("read-only"));
        assert!(!hints.contains("start-standalone"));
    }

    #[test]
    fn ready_summary_points_to_child_app_loop() {
        let root_workspace = RootWorkspaceProbe {
            state: "attached",
            path: Some("/workspace".into()),
            compose_file: PathProbe {
                label: "root docker-compose.yml",
                path: Some("/workspace/docker-compose.yml".into()),
                state: "present",
                detail: "file exists".into(),
            },
        };
        let files = FileProbes {
            santi_link_auth: PathProbe {
                label: "santi-link auth.json",
                path: Some("/workspace/modules/santi-link/auth.json".into()),
                state: "present",
                detail: "file exists".into(),
            },
        };
        let services = vec![ServiceProbe {
            name: "santi",
            source: "compose-default",
            env_var: Some("SANTI_BASE_URL"),
            base_url: "http://127.0.0.1:18081".into(),
            health_path: "/api/v1/health",
            state: "ready",
            detail: "health returned HTTP 200".into(),
        }];
        let summary = DetectSummary {
            state: "ready",
            ready: true,
            needs_action: Vec::new(),
        };
        let app_loop = AppLoopProbe {
            state: "stopped-clean",
            detail: "no stamped app-loop processes or residue found".into(),
            stamped_process_count: 0,
            stamped_processes: Vec::new(),
            residue: missing_residue(),
        };

        let hints =
            operation_hints(&root_workspace, &files, &services, &summary, &app_loop).join("\n");

        assert!(hints.contains("stim-dev restart"));
        assert!(!hints.contains("docker compose up"));
    }

    #[test]
    fn residue_without_processes_suggests_reset_before_clean_restart() {
        let root_workspace = RootWorkspaceProbe {
            state: "attached",
            path: Some("/workspace".into()),
            compose_file: PathProbe {
                label: "root docker-compose.yml",
                path: Some("/workspace/docker-compose.yml".into()),
                state: "present",
                detail: "file exists".into(),
            },
        };
        let files = FileProbes {
            santi_link_auth: PathProbe {
                label: "santi-link auth.json",
                path: Some("/workspace/modules/santi-link/auth.json".into()),
                state: "present",
                detail: "file exists".into(),
            },
        };
        let services = vec![ServiceProbe {
            name: "santi",
            source: "compose-default",
            env_var: Some("SANTI_BASE_URL"),
            base_url: "http://127.0.0.1:18081".into(),
            health_path: "/api/v1/health",
            state: "ready",
            detail: "health returned HTTP 200".into(),
        }];
        let summary = DetectSummary {
            state: "ready",
            ready: true,
            needs_action: Vec::new(),
        };
        let app_loop = AppLoopProbe {
            state: "stopped-with-residue",
            detail: "no stamped app-loop processes found, but bridge/log/lock residue exists"
                .into(),
            stamped_process_count: 0,
            stamped_processes: Vec::new(),
            residue: AppLoopResidueProbe {
                bridges: present_dir("app-loop bridges"),
                logs: missing_dir("app-loop logs"),
                locks: missing_dir("app-loop locks"),
            },
        };

        let hints =
            operation_hints(&root_workspace, &files, &services, &summary, &app_loop).join("\n");

        assert!(hints.contains("stim-dev reset"));
        assert!(hints.contains("then stim-dev restart"));
    }

    fn missing_residue() -> AppLoopResidueProbe {
        AppLoopResidueProbe {
            bridges: missing_dir("app-loop bridges"),
            logs: missing_dir("app-loop logs"),
            locks: missing_dir("app-loop locks"),
        }
    }

    fn present_dir(label: &'static str) -> PathProbe {
        PathProbe {
            label,
            path: Some(format!("/workspace/{label}")),
            state: "present",
            detail: "directory exists".into(),
        }
    }

    fn missing_dir(label: &'static str) -> PathProbe {
        PathProbe {
            label,
            path: Some(format!("/workspace/{label}")),
            state: "missing",
            detail: "directory is missing".into(),
        }
    }
}
