// trance - screensaver manager and daemon launcher for the local76 ecosystem

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Register visual theme and system query callbacks for dynamically loaded screensaver plugins
    let _ = trance_api::SYSTEM_INFO_CALLBACK.set(trance_runner::toolkit::sys_info::get_system_info);
    let _ = trance_api::PALETTE_CALLBACK.set(trance_runner::toolkit::sys_info::query_current_palette);
    let _ = trance_api::MONITOR_BOUNDS_CALLBACK.set(trance_runner::toolkit::sys_info::get_primary_monitor_bounds);
    let _ = trance_api::IS_SECONDARY_MONITOR_CALLBACK.set(trance_runner::toolkit::sys_info::is_secondary_monitor);

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        let sub = &args[1];

        // 1. Doctor command
        if sub == "doctor" || sub == "--doctor" {
            let do_fix = args.iter().any(|a| a == "--fix" || a == "-f");
            println!("trance doctor: running diagnostics (fix: {})...", do_fix);
            trance_tui::run_diagnostics(do_fix)?;
            return Ok(());
        }

        // 2. Daemon command (split out into trance-daemon)
        if sub == "daemon" || sub == "--daemon" {
            eprintln!("info: daemon service has been split out into 'trance-daemon'. Please run 'trance-daemon' instead.");
            std::process::exit(1);
        }

        // 3. Ad-hoc list command
        if sub == "list" || sub == "--list" || sub == "-l" {
            println!("available screensavers:");
            for saver in trance_tui::list_screensavers() {
                println!("  - {}", saver);
            }
            return Ok(());
        }

        // 4. Start command
        if sub == "start" || sub == "--start" {
            if args.len() < 3 {
                eprintln!("error: missing screensaver name.\nusage: trance start <name>");
                std::process::exit(1);
            }
            let name = &args[2];
            println!("launching screensaver '{}'...", name);
            trance_tui::start_screensaver(name)?;
            return Ok(());
        }

        // 5. Stop command
        if sub == "stop" || sub == "--stop" {
            println!("stopping active screensavers...");
            trance_tui::stop_screensavers()?;
            return Ok(());
        }

        // 6. Run-plugin command (internal backend loader)
        if sub == "run-plugin" {
            if args.len() < 3 {
                eprintln!("error: missing plugin path.\nusage: trance run-plugin <path>");
                std::process::exit(1);
            }
            let path = &args[2];
            match trance_runner::trance_runner::run_plugin_fullscreen(path) {
                Ok(code) => std::process::exit(code as i32),
                Err(e) => {
                    eprintln!("failed to execute screensaver plugin: {}", e);
                    std::process::exit(1);
                }
            }
        }

        // 7. Help command
        if sub == "--help" || sub == "-h" {
            print_help();
            return Ok(());
        }

        eprintln!(
            "unknown command: '{}'\nuse 'trance --help' for usage instructions.",
            sub
        );
        std::process::exit(1);
    }

    // Default TUI execution path
    println!("trance: launching interactive tui manager (press q to quit)...");
    trance_tui::app::run_app()
}

fn print_help() {
    println!(
        "trance — screensaver manager and daemon for local76

usage:
  trance                            start the interactive management tui
  trance daemon | --daemon          start the background idle daemon
  trance doctor | --doctor          run system checks (add --fix to repair)
  trance list                       list allowed screensavers
  trance start <name>               instantly run a screensaver
  trance stop                       stop any running screensavers
  trance --help | -h                show this help message"
    );
}
