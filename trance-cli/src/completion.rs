// SPDX-License-Identifier: MIT

//! Shell autocompletion script generator module.
//!
//! This module outputs autocomplete definitions for Bash and Zsh
//! so users can leverage Tab completion for subcommands and options.
//!
//! To register:
//! - Bash: `source <(trance completion bash)`
//! - Zsh: `source <(trance completion zsh)`

pub fn handle_completion(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: trance completion bash | zsh".into());
    }

    match args[0].as_str() {
        // Output Bash completion script
        "bash" => {
            let script = include_str!("completions/trance.bash");
            println!("{script}");
            Ok(())
        }
        // Output Zsh completion script
        "zsh" => {
            let script = include_str!("completions/trance.zsh");
            println!("{script}");
            Ok(())
        }
        _ => Err(format!(
            "unsupported shell '{}'; please specify 'bash' or 'zsh'",
            args[0]
        )),
    }
}
