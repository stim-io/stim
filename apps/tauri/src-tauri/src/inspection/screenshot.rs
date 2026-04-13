use std::{fs, path::PathBuf, process::Command};

use tauri::{Runtime, WebviewWindow};

use stim_shared::{
    inspection::{ScreenshotFailureReason, ScreenshotResult},
    paths::main_window_screenshots_dir,
};

pub fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");

    format!("{}-{:03}", duration.as_secs(), duration.subsec_millis())
}

fn sanitize_label(label: Option<&str>) -> Option<String> {
    label.and_then(|value| {
        let sanitized = value
            .trim()
            .to_lowercase()
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string();

        if sanitized.is_empty() {
            None
        } else {
            Some(sanitized)
        }
    })
}

pub fn capture_main_window_screenshot<R: Runtime>(
    window: Option<&WebviewWindow<R>>,
    label: Option<&str>,
) -> ScreenshotResult {
    let Some(window) = window else {
        return ScreenshotResult::Failure {
            reason: ScreenshotFailureReason::NoMainWindow,
        };
    };

    #[cfg(target_os = "macos")]
    {
        return capture_on_macos(window, label);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        let _ = label;
        ScreenshotResult::Failure {
            reason: ScreenshotFailureReason::UnsupportedPlatform,
        }
    }
}

#[cfg(target_os = "macos")]
fn capture_on_macos<R: Runtime>(
    window: &WebviewWindow<R>,
    label: Option<&str>,
) -> ScreenshotResult {
    use objc2_app_kit::NSWindow;

    let ns_window = match window.ns_window() {
        Ok(value) => value,
        Err(_) => {
            return ScreenshotResult::Failure {
                reason: ScreenshotFailureReason::CaptureFailed,
            }
        }
    };

    let window_ref: &NSWindow = unsafe { &*ns_window.cast() };
    let window_number = window_ref.windowNumber();
    let captured_at = timestamp_now();
    let label = sanitize_label(label);
    let file_path = screenshot_file_path(&captured_at, label.as_deref());

    if let Err(_) = fs::create_dir_all(main_window_screenshots_dir()) {
        return ScreenshotResult::Failure {
            reason: ScreenshotFailureReason::CaptureFailed,
        };
    }

    let status = Command::new("screencapture")
        .args([
            "-x",
            "-o",
            "-l",
            &window_number.to_string(),
            file_path.to_string_lossy().as_ref(),
        ])
        .status();

    match status {
        Ok(status) if status.success() => ScreenshotResult::Success {
            path: file_path.to_string_lossy().to_string(),
            captured_at,
            label,
        },
        _ => ScreenshotResult::Failure {
            reason: ScreenshotFailureReason::CaptureFailed,
        },
    }
}

fn screenshot_file_path(captured_at: &str, label: Option<&str>) -> PathBuf {
    let suffix = label.map(|value| format!("-{value}")).unwrap_or_default();
    main_window_screenshots_dir().join(format!("{captured_at}{suffix}.png"))
}
