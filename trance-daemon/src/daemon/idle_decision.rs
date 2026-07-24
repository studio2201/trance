//! Pure presentation policy decisions (no Wayland). Unit-testable.

/// Desired presentation transition for one policy tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentationDecision {
    /// No change required.
    Hold,
    /// Stop any active presentation.
    Stop {
        /// Clear forced preview name (lock/inhibit/stale surface).
        clear_preview: bool,
    },
    /// Start (or switch to) this saver.
    Start {
        name: String,
        reason: &'static str,
    },
}

/// Inputs for one idle-policy evaluation.
#[derive(Debug, Clone, Copy)]
pub struct IdlePolicyInput<'a> {
    pub is_active: bool,
    pub surface_visible: bool,
    pub current_saver: &'a str,
    pub preview_name: Option<&'a str>,
    pub idle_enabled: bool,
    pub system_idle: bool,
    pub session_locked: bool,
    pub inhibited: bool,
}

/// Decide the next presentation action from pure inputs.
///
/// `idle_saver_name` is used only when starting due to system idle.
pub fn decide_presentation(
    input: IdlePolicyInput<'_>,
    idle_saver_name: &str,
) -> PresentationDecision {
    if input.is_active && !input.surface_visible {
        return PresentationDecision::Stop {
            clear_preview: true,
        };
    }

    if input.session_locked || input.inhibited {
        if input.is_active || input.preview_name.is_some() {
            return PresentationDecision::Stop {
                clear_preview: true,
            };
        }
        return PresentationDecision::Hold;
    }

    if let Some(name) = input.preview_name {
        if input.is_active && input.current_saver != name {
            return PresentationDecision::Start {
                name: name.to_string(),
                reason: "preview",
            };
        }
        if !input.is_active {
            return PresentationDecision::Start {
                name: name.to_string(),
                reason: "preview",
            };
        }
        return PresentationDecision::Hold;
    }

    if input.idle_enabled && input.system_idle && !input.is_active {
        return PresentationDecision::Start {
            name: idle_saver_name.to_string(),
            reason: "idle",
        };
    }

    // Activity only stops idle-driven presentation, not an explicit preview.
    if input.is_active && !input.system_idle {
        return PresentationDecision::Stop {
            clear_preview: false,
        };
    }

    if !input.idle_enabled && input.is_active {
        return PresentationDecision::Stop {
            clear_preview: false,
        };
    }

    PresentationDecision::Hold
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn hold_when_idle_already_active() {
        let mut i = base();
        i.is_active = true;
        i.system_idle = true;
        i.current_saver = "beams";
        assert_eq!(decide_presentation(i, "beams"), PresentationDecision::Hold);
    }
}
