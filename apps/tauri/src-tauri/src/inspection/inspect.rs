use tauri::{AppHandle, Manager, Monitor, Runtime, WebviewWindow};

use stim_shared::inspection::{
    AppInspectSnapshot, InspectFailureReason, InspectResult, InspectSnapshot,
    MonitorInspectSnapshot, PhysicalPositionSnapshot, PhysicalRectSnapshot, PhysicalSizeSnapshot,
    WindowInspectSnapshot,
};

pub fn inspect_main_window<R: Runtime>(app: &AppHandle<R>) -> InspectResult {
    let Some(window) = app.get_webview_window("main") else {
        return InspectResult::Failure {
            reason: InspectFailureReason::NoMainWindow,
        };
    };

    let snapshot = match build_inspect_snapshot(app, &window) {
        Ok(snapshot) => snapshot,
        Err(_) => {
            return InspectResult::Failure {
                reason: InspectFailureReason::InspectFailed,
            };
        }
    };

    InspectResult::Success { snapshot }
}

fn build_inspect_snapshot<R: Runtime>(
    app: &AppHandle<R>,
    window: &WebviewWindow<R>,
) -> tauri::Result<InspectSnapshot> {
    Ok(InspectSnapshot {
        inspected_at: crate::inspection::screenshot::timestamp_now(),
        app: AppInspectSnapshot {
            name: app.package_info().name.clone(),
            version: app.package_info().version.to_string(),
            crate_name: app.package_info().crate_name.to_string(),
            expected_renderer_origin: stim_shared::RENDERER_DEV_URL.to_string(),
        },
        window: build_window_snapshot(window)?,
        current_monitor: window.current_monitor()?.map(snapshot_monitor),
        primary_monitor: window.primary_monitor()?.map(snapshot_monitor),
        available_monitor_count: window.available_monitors()?.len(),
    })
}

fn build_window_snapshot<R: Runtime>(
    window: &WebviewWindow<R>,
) -> tauri::Result<WindowInspectSnapshot> {
    let url = window.url()?.to_string();
    let inner_size = window.inner_size()?;
    let outer_size = window.outer_size()?;
    let outer_position = window.outer_position()?;

    Ok(WindowInspectSnapshot {
        label: window.label().to_string(),
        title: window.title()?,
        matches_expected_renderer_origin: url.starts_with(stim_shared::RENDERER_DEV_URL),
        url,
        scale_factor: window.scale_factor()?,
        inner_size: PhysicalSizeSnapshot {
            width: inner_size.width,
            height: inner_size.height,
        },
        outer_size: PhysicalSizeSnapshot {
            width: outer_size.width,
            height: outer_size.height,
        },
        outer_position: PhysicalPositionSnapshot {
            x: outer_position.x,
            y: outer_position.y,
        },
        is_visible: window.is_visible()?,
        is_focused: window.is_focused()?,
        is_minimized: window.is_minimized()?,
        is_maximized: window.is_maximized()?,
        is_fullscreen: window.is_fullscreen()?,
        is_decorated: window.is_decorated()?,
        is_resizable: window.is_resizable()?,
        is_enabled: window.is_enabled()?,
    })
}

fn snapshot_monitor(monitor: Monitor) -> MonitorInspectSnapshot {
    let size = monitor.size();
    let position = monitor.position();
    let work_area = monitor.work_area();

    MonitorInspectSnapshot {
        name: monitor.name().cloned(),
        scale_factor: monitor.scale_factor(),
        size: PhysicalSizeSnapshot {
            width: size.width,
            height: size.height,
        },
        position: PhysicalPositionSnapshot {
            x: position.x,
            y: position.y,
        },
        work_area: PhysicalRectSnapshot {
            x: work_area.position.x,
            y: work_area.position.y,
            width: work_area.size.width,
            height: work_area.size.height,
        },
    }
}
