// SPDX-License-Identifier: MIT

use super::*;
use crate::config::DaemonConfig;

static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn test_controller() -> (
    DaemonController,
    std::path::PathBuf,
    std::sync::MutexGuard<'static, ()>,
) {
    let guard = TEST_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let temp = std::env::temp_dir().join(format!(
        "trance-daemon-cmd-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&temp).expect("create temp config dir for command tests");
    // SAFETY: tests hold TEST_MUTEX; only this suite mutates XDG_CONFIG_HOME.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", &temp);
    }
    let controller = DaemonController::new(DaemonConfig::default());
    (controller, temp, guard)
}

#[test]
fn enable_sets_idle_true() {
    let (c, _tmp, _guard) = test_controller();
    c.apply_command(DaemonCommand::Enable)
        .expect("Enable should succeed");
    assert!(
        c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .idle_enabled
    );
}

#[test]
fn disable_sets_idle_false() {
    let (c, _tmp, _guard) = test_controller();
    c.apply_command(DaemonCommand::Disable)
        .expect("Disable should succeed");
    assert!(
        !c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .idle_enabled
    );
}

#[test]
fn set_timeout_validates_range() {
    let (c, _tmp, _guard) = test_controller();
    assert!(c.apply_command(DaemonCommand::SetTimeout(0)).is_err());
    assert!(c.apply_command(DaemonCommand::SetTimeout(241)).is_err());
    assert!(c.apply_command(DaemonCommand::SetTimeout(10)).is_ok());
    assert_eq!(
        c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .idle_timeout_mins,
        10
    );
}

#[test]
fn set_timeout_accepts_boundaries() {
    let (c, _tmp, _guard) = test_controller();
    assert!(c.apply_command(DaemonCommand::SetTimeout(1)).is_ok());
    assert!(c.apply_command(DaemonCommand::SetTimeout(240)).is_ok());
    assert_eq!(
        c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .idle_timeout_mins,
        240
    );
}

#[test]
fn set_render_scale_zero_normalizes_to_none() {
    let (c, _tmp, _guard) = test_controller();
    c.apply_command(DaemonCommand::SetRenderScale(Some(0.0)))
        .expect("zero scale should normalize");
    assert!(
        c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .render_scale
            .is_none()
    );
}

#[test]
fn set_render_scale_rejects_out_of_range() {
    let (c, _tmp, _guard) = test_controller();
    assert!(
        c.apply_command(DaemonCommand::SetRenderScale(Some(2.0)))
            .is_err()
    );
    assert!(
        c.apply_command(DaemonCommand::SetRenderScale(Some(0.1)))
            .is_err()
    );
}

#[test]
fn set_render_scale_accepts_in_range() {
    let (c, _tmp, _guard) = test_controller();
    c.apply_command(DaemonCommand::SetRenderScale(Some(0.5)))
        .expect("0.5 scale in range");
    assert_eq!(
        c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .render_scale,
        Some(0.5)
    );
}

#[test]
fn set_render_scale_accepts_none() {
    let (c, _tmp, _guard) = test_controller();
    c.apply_command(DaemonCommand::SetRenderScale(None))
        .expect("None scale accepted");
    assert!(
        c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .render_scale
            .is_none()
    );
}

#[test]
fn set_show_fps_overlay_toggles() {
    let (c, _tmp, _guard) = test_controller();
    c.apply_command(DaemonCommand::SetShowFpsOverlay(true))
        .expect("enable fps overlay");
    assert!(
        c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .show_fps_overlay
    );
    c.apply_command(DaemonCommand::SetShowFpsOverlay(false))
        .expect("disable fps overlay");
    assert!(
        !c.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .show_fps_overlay
    );
}

#[test]
fn preview_and_stop_are_no_ops() {
    let (c, _tmp, _guard) = test_controller();
    assert!(
        c.apply_command(DaemonCommand::Preview("beams".into()))
            .is_ok()
    );
    assert!(c.apply_command(DaemonCommand::StopPresentation).is_ok());
}

#[test]
fn mark_dirty_sets_status_dirty_flag() {
    let (c, _tmp, _guard) = test_controller();
    let _ = c.take_dirty();
    c.mark_dirty();
    assert!(c.take_dirty());
    assert!(!c.take_dirty());
}

#[test]
fn validate_idle_timeout_bounds() {
    assert!(validate_idle_timeout(0).is_err());
    assert!(validate_idle_timeout(241).is_err());
    assert!(validate_idle_timeout(1).is_ok());
    assert!(validate_idle_timeout(240).is_ok());
    assert!(validate_idle_timeout(120).is_ok());
}

#[test]
fn validate_render_scale_in_range() {
    assert!(validate_render_scale(0.25).is_ok());
    assert!(validate_render_scale(1.0).is_ok());
    assert!(validate_render_scale(0.5).is_ok());
    assert!(validate_render_scale(0.24).is_err());
    assert!(validate_render_scale(1.01).is_err());
    assert!(validate_render_scale(f32::NAN).is_err());
}

#[test]
fn normalize_render_scale_handles_edges() {
    assert!(normalize_render_scale(None).expect("None ok").is_none());
    assert!(normalize_render_scale(Some(0.0)).expect("0.0 ok").is_none());
    assert!(
        normalize_render_scale(Some(-1.0))
            .expect("-1.0 ok")
            .is_none()
    );
    assert_eq!(
        normalize_render_scale(Some(0.5)).expect("0.5 ok"),
        Some(0.5)
    );
    assert!(normalize_render_scale(Some(2.0)).is_err());
}
