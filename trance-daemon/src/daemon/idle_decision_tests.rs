// SPDX-License-Identifier: Apache-2.0

//! Unit tests for pure idle presentation policy.

use super::{IdlePolicyInput, PresentationDecision, decide_presentation};

fn base() -> IdlePolicyInput<'static> {
    IdlePolicyInput {
        is_active: false,
        surface_visible: true,
        current_saver: "",
        preview_name: None,
        idle_enabled: true,
        system_idle: false,
        session_locked: false,
        inhibited: false,
    }
}

#[test]
fn idle_starts_saver() {
    let mut i = base();
    i.system_idle = true;
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Start {
            name: "beams".into(),
            reason: "idle",
        }
    );
}

#[test]
fn activity_stops_active_idle() {
    let mut i = base();
    i.is_active = true;
    i.current_saver = "beams";
    i.system_idle = false;
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Stop {
            clear_preview: false,
        }
    );
}

#[test]
fn preview_survives_activity() {
    let mut i = base();
    i.is_active = true;
    i.current_saver = "storm";
    i.preview_name = Some("storm");
    i.system_idle = false;
    assert_eq!(decide_presentation(i, "beams"), PresentationDecision::Hold);
}

#[test]
fn lock_stops_and_clears_preview() {
    let mut i = base();
    i.is_active = true;
    i.session_locked = true;
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Stop {
            clear_preview: true,
        }
    );
}

#[test]
fn inhibit_stops() {
    let mut i = base();
    i.is_active = true;
    i.inhibited = true;
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Stop {
            clear_preview: true,
        }
    );
}

#[test]
fn preview_overrides_idle() {
    let mut i = base();
    i.system_idle = true;
    i.preview_name = Some("storm");
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Start {
            name: "storm".into(),
            reason: "preview",
        }
    );
}

#[test]
fn preview_switch_restarts() {
    let mut i = base();
    i.is_active = true;
    i.current_saver = "beams";
    i.preview_name = Some("storm");
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Start {
            name: "storm".into(),
            reason: "preview",
        }
    );
}

#[test]
fn disabled_stops_active() {
    let mut i = base();
    i.is_active = true;
    i.idle_enabled = false;
    i.system_idle = true;
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Stop {
            clear_preview: false,
        }
    );
}

#[test]
fn stale_surface_stops() {
    let mut i = base();
    i.is_active = true;
    i.surface_visible = false;
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Stop {
            clear_preview: true,
        }
    );
}

#[test]
fn lock_clears_preview_even_when_inactive() {
    let mut i = base();
    i.preview_name = Some("storm");
    i.session_locked = true;
    assert_eq!(
        decide_presentation(i, "beams"),
        PresentationDecision::Stop {
            clear_preview: true,
        }
    );
}

#[test]
fn idle_disabled_does_not_start() {
    let mut i = base();
    i.system_idle = true;
    i.idle_enabled = false;
    assert_eq!(decide_presentation(i, "beams"), PresentationDecision::Hold);
}

#[test]
fn hold_when_idle_already_active() {
    let mut i = base();
    i.is_active = true;
    i.system_idle = true;
    i.current_saver = "beams";
    assert_eq!(decide_presentation(i, "beams"), PresentationDecision::Hold);
}

#[test]
fn lock_and_inhibit_hold_when_inactive_no_preview() {
    let mut i = base();
    i.session_locked = true;
    assert_eq!(decide_presentation(i, "beams"), PresentationDecision::Hold);
    i.session_locked = false;
    i.inhibited = true;
    assert_eq!(decide_presentation(i, "beams"), PresentationDecision::Hold);
}

#[test]
fn hold_when_not_idle_and_inactive() {
    let i = base();
    assert_eq!(decide_presentation(i, "beams"), PresentationDecision::Hold);
}
