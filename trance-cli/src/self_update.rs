// SPDX-License-Identifier: MIT

use std::process::Command;

pub fn handle_self_update() -> Result<(), String> {
    println!("Checking for updates in APT repository...");

    let output = Command::new("apt-cache")
        .args(["policy", "trance"])
        .output();

    match output {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut installed = None;
            let mut candidate = None;
            for line in text.lines() {
                let line = line.trim();
                if line.starts_with("Installed:") {
                    installed = Some(line["Installed:".len()..].trim().to_string());
                } else if line.starts_with("Candidate:") {
                    candidate = Some(line["Candidate:".len()..].trim().to_string());
                }
            }

            match (installed, candidate) {
                (Some(inst), Some(cand)) => {
                    if inst == "(none)" {
                        println!(" [!] Trance is not currently installed as a system package.");
                        println!("     -> Fix: Install it with: sudo apt install trance");
                    } else if inst != cand {
                        println!(" [!] A new version is available: {inst} -> {cand}");
                        println!("     -> Run: sudo apt update && sudo apt install --only-upgrade trance");
                    } else {
                        println!(" [✔] Trance is already up to date (version {inst}).");
                    }
                }
                _ => {
                    println!(" [!] Could not determine installed package policy for 'trance'.");
                    println!("     -> Note: Ensure the local76 APT repository is registered.");
                }
            }
        }
        Err(_) => {
            println!(" [!] 'apt-cache' command not available or failed.");
            println!("     -> Note: Package updates are managed by APT on this system.");
        }
    }
    Ok(())
}
