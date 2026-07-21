use anyhow::Result;
use std::process::Command;

use super::doctor_checks::{
    CheckResult, check_config_parses, check_dbus, check_fonts, check_package_install,
    check_running_pid, check_systemd_service, check_wayland,
};

/// Run diagnostics. When `fix` is true, attempt to reload/enable/restart the
/// user unit so upgrades do not require remembering systemctl flags.
pub fn run_doctor(fix: bool) -> Result<()> {
    println!("==========================================");
    println!("Trance System Diagnostics (Doctor)");
    println!("==========================================");

    if fix {
        fix_user_service()?;
        println!();
    }

    let results = vec![
        check_wayland(),
        check_dbus(),
        check_systemd_service(),
        check_running_pid(),
        check_config_parses(),
        check_fonts(),
        check_package_install(),
    ];
    print_results(&results);
    if !results.iter().all(|r| r.passed) {
        if !fix {
            println!("Hint: try  trance doctor --fix  to reload/enable the user service.");
        }
        std::process::exit(1);
    }
    Ok(())
}

/// Best-effort recovery after package upgrade or a dead session service.
fn fix_user_service() -> Result<()> {
    println!("--fix: reloading and ensuring trance-daemon user service...");

    let reload = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .status();
    if let Ok(st) = reload
        && st.success()
    {
        println!("  ✓ systemctl --user daemon-reload");
    } else {
        println!("  ! systemctl daemon-reload returned an error or was missing");
    }

    let enable = Command::new("systemctl")
        .args(["--user", "enable", "--now", "trance-daemon.service"])
        .status();
    if let Ok(st) = enable
        && st.success()
    {
        println!("  ✓ systemctl --user enable --now trance-daemon.service");
    } else {
        println!("  ! enable --now failed; attempting restart...");
        let _ = Command::new("systemctl")
            .args(["--user", "restart", "trance-daemon.service"])
            .status();
    }
    Ok(())
}

fn print_results(results: &[CheckResult]) {
    println!("==========================================");
    for result in results {
        let marker = if result.passed { "✓" } else { "✗" };
        println!("  [{marker}] {}: {}", result.name, result.detail);
    }
    println!("==========================================");
    if results.iter().all(|r| r.passed) {
        println!("Diagnostics complete: ALL SYSTEMS NOMINAL.");
    } else {
        println!("Diagnostics complete: PROBLEMS DETECTED.");
        println!("Diagnostics check failed. Please resolve the issues marked with [✗].");
    }
}
