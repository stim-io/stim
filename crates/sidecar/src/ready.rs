use std::{
    fmt,
    io::{BufRead, BufReader, Read},
    sync::mpsc,
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::identity::SidecarStamp;

pub const READY_LINE_KIND: &str = "stim-sidecar-ready";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidecarReadyLine {
    pub kind: String,
    pub stamp: SidecarStamp,
    pub role: String,
    pub instance_id: String,
    pub endpoint: Option<String>,
    pub ready_at: String,
}

impl SidecarReadyLine {
    pub fn new(
        stamp: SidecarStamp,
        role: String,
        instance_id: String,
        endpoint: Option<String>,
        ready_at: String,
    ) -> Self {
        Self {
            kind: READY_LINE_KIND.to_string(),
            stamp,
            role,
            instance_id,
            endpoint,
            ready_at,
        }
    }

    pub fn is_ready_line(&self) -> bool {
        self.kind == READY_LINE_KIND
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadyLineWaitError {
    ExitedBeforeReady,
    ReadFailed(String),
    TimedOut,
}

impl fmt::Display for ReadyLineWaitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExitedBeforeReady => formatter.write_str("sidecar exited before ready"),
            Self::ReadFailed(error) => {
                write!(formatter, "failed to read sidecar ready line: {error}")
            }
            Self::TimedOut => formatter.write_str("timed out waiting for sidecar ready line"),
        }
    }
}

impl std::error::Error for ReadyLineWaitError {}

pub fn wait_for_ready_line<R>(
    reader: R,
    timeout: Duration,
) -> Result<SidecarReadyLine, ReadyLineWaitError>
where
    R: Read + Send + 'static,
{
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let _ = sender.send(Err(ReadyLineWaitError::ExitedBeforeReady));
                    return;
                }
                Ok(_) => {
                    if let Ok(ready) = serde_json::from_str::<SidecarReadyLine>(line.trim()) {
                        let _ = sender.send(Ok(ready));
                        drain_remaining_lines(reader);
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(ReadyLineWaitError::ReadFailed(error.to_string())));
                    return;
                }
            }
        }
    });

    receiver
        .recv_timeout(timeout)
        .map_err(|_| ReadyLineWaitError::TimedOut)?
}

fn drain_remaining_lines<R>(mut reader: BufReader<R>)
where
    R: Read,
{
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|read| read > 0) {
        line.clear();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        identity::{SidecarMode, SidecarStamp, SOURCE_APP_PACKAGED},
        ready::{wait_for_ready_line, SidecarReadyLine, READY_LINE_KIND},
    };
    use std::{io::Cursor, time::Duration};

    #[test]
    fn ready_line_has_stable_kind_and_optional_endpoint() {
        let line = SidecarReadyLine::new(
            SidecarStamp {
                app: "controller".into(),
                namespace: "default".into(),
                mode: SidecarMode::Runtime,
                source: SOURCE_APP_PACKAGED.into(),
            },
            "controller-runtime".into(),
            "controller-1".into(),
            Some("http://127.0.0.1:43123".into()),
            "2026-04-27T00:00:00Z".into(),
        );

        assert_eq!(line.kind, READY_LINE_KIND);
        assert!(line.is_ready_line());
        assert_eq!(line.endpoint.as_deref(), Some("http://127.0.0.1:43123"));

        let value = serde_json::to_value(&line).unwrap();
        assert!(value.get("stamp").is_some());
        assert!(value.get("identity").is_none());
        assert_eq!(value["role"], "controller-runtime");
        assert_eq!(value["instance_id"], "controller-1");
    }

    #[test]
    fn waits_for_first_ready_line_in_stream() {
        let content = concat!(
            "compile noise\n",
            "{\"kind\":\"stim-sidecar-ready\",\"stamp\":{\"app\":\"controller\",\"namespace\":\"default\",\"mode\":\"runtime\",\"source\":\"app:packaged\"},\"role\":\"controller-runtime\",\"instance_id\":\"controller-1\",\"endpoint\":\"http://127.0.0.1:43123\",\"ready_at\":\"2026-04-27T00:00:00Z\"}\n"
        );
        let ready = wait_for_ready_line(Cursor::new(content), Duration::from_secs(1)).unwrap();

        assert_eq!(ready.stamp.app, "controller");
        assert_eq!(ready.role, "controller-runtime");
    }
}
