// SPDX-License-Identifier: MIT

use std::process::ExitCode;

use trance_dbus::{TranceClient, daemon_available};

mod bug_report;
mod clean;
mod completion;
mod config;
mod doctor;
mod interactive;
mod self_update;

#[cfg(test)]
mod tests;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("trance: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help" | "help") {
        print_usage();
        return Ok(());
    }

    match args[0].as_str() {
        "doctor" => return doctor::run_doctor(),
        "clean" => return clean::handle_clean(),
        "completion" => return completion::handle_completion(&args[1..]),
        "bug-report" => return bug_report::handle_bug_report(),
        "self-update" => return self_update::handle_self_update(),
        _ => {}
    }

    let client = TranceClient::connect().map_err(|error| {
        if daemon_available() {
            format!("failed to connect to daemon: {error}")
        } else {
            "trance-daemon is not running; start it with: systemctl --user start trance-daemon"
                .into()
        }
    })?;

    match args[0].as_str() {
        "status" => cmd_status(&client, &args[1..]),
        "config" => config::handle_config(&client, &args[1..]),
        "interactive" => interactive::run_interactive(&client),
        "enable" => client.enable().map_err(map_dbus),
        "disable" => client.disable().map_err(map_dbus),
        "timeout" => cmd_timeout(&client, &args[1..]),
        "saver" => cmd_saver(&client, &args[1..]),
        "list" => cmd_list(&client),
        "preview" => cmd_preview(&client, &args[1..]),
        "stop" => client.stop_preview().map_err(map_dbus),
        "fps-overlay" => cmd_fps_overlay(&client, &args[1..]),
        "render-scale" => cmd_render_scale(&client, &args[1..]),
        _ => {
            print_usage();
            Err(format!("unknown command: {}", args[0]))
        }
    }
}

fn cmd_status(client: &TranceClient, args: &[String]) -> Result<(), String> {
    let status = client.get_status().map_err(map_dbus)?;
    if args.first().map(String::as_str) == Some("--json") {
        println!(
            "{{\"running\":{},\"idle_enabled\":{},\"idle_timeout_mins\":{},\"active_saver\":\"{}\",\"gpu_enabled\":{},\"show_fps_overlay\":{},\"render_scale\":\"{}\",\"presentation_active\":{},\"preview_active\":{},\"current_saver\":\"{}\",\"system_idle\":{},\"session_locked\":{},\"inhibited\":{}}}",
            status.running,
            status.idle_enabled,
            status.idle_timeout_mins,
            status.active_saver,
            status.gpu_enabled,
            status.show_fps_overlay,
            status.render_scale,
            status.presentation_active,
            status.preview_active,
            status.current_saver,
            status.system_idle,
            status.session_locked,
            status.inhibited
        );
    } else {
        println!("running:              {}", status.running);
        println!("idle_enabled:         {}", status.idle_enabled);
        println!("idle_timeout_mins:    {}", status.idle_timeout_mins);
        println!(
            "active_saver:         {}",
            display_saver(&status.active_saver)
        );
        println!("gpu_enabled:          {}", status.gpu_enabled);
        println!("show_fps_overlay:     {}", status.show_fps_overlay);
        println!(
            "render_scale:         {}",
            if status.render_scale.is_empty() {
                "default"
            } else {
                &status.render_scale
            }
        );
        println!("presentation_active:  {}", status.presentation_active);
        println!("preview_active:       {}", status.preview_active);
        println!("current_saver:        {}", status.current_saver);
        println!("system_idle:          {}", status.system_idle);
        println!("session_locked:       {}", status.session_locked);
        println!("inhibited:            {}", status.inhibited);
    }
    Ok(())
}

fn cmd_timeout(client: &TranceClient, args: &[String]) -> Result<(), String> {
    let minutes = match args {
        [value] => value
            .parse::<u32>()
            .map_err(|_| "timeout requires a number of minutes (1–240)".to_string())?,
        _ => return Err("usage: trance timeout <minutes>".into()),
    };
    client.set_timeout(minutes).map_err(map_dbus)
}

fn cmd_saver(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args {
        [cmd, name] if cmd == "set" => {
            let dbus_name = if name == "random" { "" } else { name.as_str() };
            client.set_saver(dbus_name).map_err(map_dbus)
        }
        [cmd] if cmd == "list" => cmd_list(client),
        _ => Err("usage: trance saver set <name|random> | trance saver list".into()),
    }
}

fn cmd_list(client: &TranceClient) -> Result<(), String> {
    let savers = client.list_savers().map_err(map_dbus)?;
    for saver in savers {
        println!("{saver}");
    }
    Ok(())
}

fn cmd_preview(client: &TranceClient, args: &[String]) -> Result<(), String> {
    let name = args
        .first()
        .ok_or_else(|| "usage: trance preview <saver>".to_string())?;
    client.preview(name).map_err(map_dbus)
}

fn cmd_fps_overlay(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().map_err(map_dbus)?;
            println!(
                "fps overlay: {}",
                if status.show_fps_overlay { "on" } else { "off" }
            );
            Ok(())
        }
        Some("on") => client.set_show_fps_overlay(true).map_err(map_dbus),
        Some("off") => client.set_show_fps_overlay(false).map_err(map_dbus),
        Some(value) => Err(format!(
            "unknown fps-overlay subcommand: {value} (use on, off, status)"
        )),
    }
}

fn cmd_render_scale(client: &TranceClient, args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("status") => {
            let status = client.get_status().map_err(map_dbus)?;
            println!(
                "render scale: {}",
                if status.render_scale.is_empty() {
                    "default"
                } else {
                    &status.render_scale
                }
            );
            Ok(())
        }
        Some("default") => client.set_render_scale(0.0).map_err(map_dbus),
        Some(value) => {
            let scale = value
                .parse::<f32>()
                .map_err(|_| "render-scale requires a number between 0.25 and 1.0".to_string())?;
            if !(0.25..=1.0).contains(&scale) {
                return Err("render-scale must be between 0.25 and 1.0".into());
            }
            client.set_render_scale(scale).map_err(map_dbus)
        }
    }
}

fn display_saver(name: &str) -> String {
    if name.is_empty() {
        "random".into()
    } else {
        name.to_string()
    }
}

fn map_dbus(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn print_usage() {
    eprintln!(
        "Usage: trance <command> [args]\n\
         \n\
         Commands:\n\
           status [--json]        Show daemon state\n\
           enable | disable       Toggle idle screensaver\n\
           timeout <minutes>      Set idle timeout (1–240)\n\
           saver set <name|random>\n\
           saver list | list      List installed savers\n\
           preview <saver>        Preview a screensaver now\n\
           stop                   Stop preview or idle presentation\n\
           fps-overlay on|off|status  Toggle on-screen FPS overlay\n\
           render-scale <0.25-1.0>|default|status  Simulation grid density (zoom)\n\
           doctor                 Run system diagnostics\n\
           config get/set/list    Unified configuration manager\n\
           completion bash/zsh    Generate shell tab-completion scripts\n\
           clean                  Clean stale runs and log caches\n\
           bug-report             Generate sanitized bug reports\n\
           self-update            Check for package updates\n\
           interactive            Open interactive console panel\n"
    );
}
