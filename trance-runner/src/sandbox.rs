// SPDX-License-Identifier: MIT

use landlock::Access;
use landlock::RulesetAttr;
use landlock::{ABI, AccessFs, Ruleset};

/// Enforces a strict Landlock filesystem sandbox on the current process,
/// locking down all filesystem access (read, write, execute).
pub fn enforce_sandbox() -> Result<(), String> {
    // Use ABI::V1 which is the baseline Landlock version supported since 5.13.
    // We handle all filesystem access rights to ensure a total lockdown.
    let ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(ABI::V1))
        .map_err(|e| format!("Failed to initialize ruleset: {e}"))?;

    let ruleset = ruleset
        .create()
        .map_err(|e| format!("Failed to create ruleset: {e}"))?;

    let status = ruleset
        .restrict_self()
        .map_err(|e| format!("Failed to enforce Landlock sandbox: {e}"))?;

    tracing::info!("Landlock filesystem sandbox enforced: {:?}", status);
    Ok(())
}
