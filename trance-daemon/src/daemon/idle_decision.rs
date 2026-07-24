// SPDX-License-Identifier: Apache-2.0

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
    Start { name: String, reason: &'static str },
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
#[path = "idle_decision_tests.rs"]
mod tests;
