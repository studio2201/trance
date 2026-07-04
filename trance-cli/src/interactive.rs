// SPDX-License-Identifier: MIT

use std::io::{self, Write};
use trance_dbus::TranceClient;

pub fn run_interactive(client: &TranceClient) -> Result<(), String> {
    loop {
        let status = client.get_status().map_err(|e| e.to_string())?;

        println!("\n==========================================");
        println!("Trance Interactive Control Panel");
        println!("==========================================");
        println!(
            " 1. Toggle Idle Activation (Current: {})",
            if status.idle_enabled {
                "ENABLED"
            } else {
                "DISABLED"
            }
        );
        println!(
            " 2. Set Idle Timeout       (Current: {} mins)",
            status.idle_timeout_mins
        );
        println!(
            " 3. Select Active Saver    (Current: {})",
            if status.active_saver.is_empty() {
                "random"
            } else {
                &status.active_saver
            }
        );
        println!(" 4. Preview a Screensaver");
        println!(
            " 5. Toggle FPS Overlay     (Current: {})",
            if status.show_fps_overlay { "ON" } else { "OFF" }
        );
        println!(" 6. Stop Current Preview / Presentation");
        println!(" 7. Exit");
        print!("\nSelect an option (1-7): ");
        io::stdout().flush().map_err(|e| e.to_string())?;

        let mut choice = String::new();
        io::stdin()
            .read_line(&mut choice)
            .map_err(|e| e.to_string())?;
        let choice = choice.trim();

        match choice {
            "1" => {
                if status.idle_enabled {
                    client.disable().map_err(|e| e.to_string())?;
                    println!("Disabled screensaver activation.");
                } else {
                    client.enable().map_err(|e| e.to_string())?;
                    println!("Enabled screensaver activation.");
                }
            }
            "2" => {
                print!("Enter new timeout (1-240 mins): ");
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut timeout_str = String::new();
                io::stdin()
                    .read_line(&mut timeout_str)
                    .map_err(|e| e.to_string())?;
                if let Ok(mins) = timeout_str.trim().parse::<u32>() {
                    if (1..=240).contains(&mins) {
                        client.set_timeout(mins).map_err(|e| e.to_string())?;
                        println!("Timeout updated to {mins} minutes.");
                    } else {
                        println!("Invalid range (must be 1-240).");
                    }
                } else {
                    println!("Invalid input.");
                }
            }
            "3" => {
                let savers = client.list_savers().map_err(|e| e.to_string())?;
                println!("\nAvailable Screensavers:");
                println!("  0. random (default)");
                for (i, s) in savers.iter().enumerate() {
                    println!("  {}. {s}", i + 1);
                }
                print!("Select a saver (0-{}): ", savers.len());
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut idx_str = String::new();
                io::stdin()
                    .read_line(&mut idx_str)
                    .map_err(|e| e.to_string())?;
                if let Ok(idx) = idx_str.trim().parse::<usize>() {
                    if idx == 0 {
                        client.set_saver("").map_err(|e| e.to_string())?;
                        println!("Active screensaver set to: random");
                    } else if idx <= savers.len() {
                        client
                            .set_saver(&savers[idx - 1])
                            .map_err(|e| e.to_string())?;
                        println!("Active screensaver set to: {}", savers[idx - 1]);
                    } else {
                        println!("Invalid choice.");
                    }
                }
            }
            "4" => {
                let savers = client.list_savers().map_err(|e| e.to_string())?;
                println!("\nChoose screensaver to preview:");
                for (i, s) in savers.iter().enumerate() {
                    println!("  {}. {s}", i + 1);
                }
                print!("Select a screensaver (1-{}): ", savers.len());
                io::stdout().flush().map_err(|e| e.to_string())?;
                let mut idx_str = String::new();
                io::stdin()
                    .read_line(&mut idx_str)
                    .map_err(|e| e.to_string())?;
                if let Ok(idx) = idx_str.trim().parse::<usize>() {
                    if idx >= 1 && idx <= savers.len() {
                        client
                            .preview(&savers[idx - 1])
                            .map_err(|e| e.to_string())?;
                        println!("Starting preview of {}...", savers[idx - 1]);
                    } else {
                        println!("Invalid choice.");
                    }
                }
            }
            "5" => {
                client
                    .set_show_fps_overlay(!status.show_fps_overlay)
                    .map_err(|e| e.to_string())?;
                println!(
                    "FPS overlay toggled to {}.",
                    if !status.show_fps_overlay {
                        "ON"
                    } else {
                        "OFF"
                    }
                );
            }
            "6" => {
                client.stop_preview().map_err(|e| e.to_string())?;
                println!("Presentation stopped.");
            }
            "7" => {
                println!("Exiting control panel.");
                break;
            }
            _ => println!("Invalid selection. Please enter a number 1-7."),
        }
    }
    Ok(())
}
