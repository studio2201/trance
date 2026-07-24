//! Secure plugin path resolution for screensaver `.so` libraries.

use std::path::{Path, PathBuf};

/// Errors that can occur during plugin loading and initialization.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("plugin name '{0}' is not in the allowlist")]
    NotAllowed(String),
    #[error("plugin path contains '..' (path traversal attempt)")]
    PathTraversal,
    #[error("invalid plugin name: {0}")]
    InvalidName(String),
    #[error("failed to load library: {0}")]
    LoadFailure(#[from] libloading::Error),
    #[error("symbol '{0}' not found in plugin")]
    SymbolMissing(&'static str),
    #[error("plugin API version {found} incompatible with host {expected}")]
    ApiVersionMismatch { found: u32, expected: u32 },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// The canonical list of allowed saver basenames.
pub const ALLOWED_SAVERS: &[&str] = &[
    "beams", "bursts", "chaos", "cosmos", "glyphs", "gnats", "radar", "storm", "hearth", "ripple",
];

/// Controls which directories [`resolve_saver_binary`] may search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaunchMode {
    /// Installed system paths only.
    Daemon,
    /// Installed paths plus local development build trees.
    Preview,
}

/// Whether `name` resolves to a built-in screensaver package.
pub fn is_allowed_saver(name: &str) -> bool {
    if name.contains('/') || name.contains('\\') {
        return false;
    }
    sanitize_saver_name(name)
        .as_deref()
        .is_some_and(|clean| ALLOWED_SAVERS.contains(&clean))
}

/// Reduce a raw name or path to a clean basename, if valid.
pub fn sanitize_saver_name(raw: &str) -> Option<String> {
    let mut stem = Path::new(raw)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(raw)
        .to_string();

    if stem.starts_with("libscreensaver_") {
        stem = stem["libscreensaver_".len()..].to_string();
    } else if stem.starts_with("lib") {
        stem = stem["lib".len()..].to_string();
    }

    if stem.starts_with("screensaver-") {
        stem = stem["screensaver-".len()..].to_string();
    }
    // Package name form: idle-saver-beams → beams
    if stem.starts_with("idle-saver-") {
        stem = stem["idle-saver-".len()..].to_string();
    }

    if !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }

    if stem.is_empty() {
        return None;
    }

    Some(stem)
}

fn dev_plugin_dirs(clean: &str) -> Vec<PathBuf> {
    if !cfg!(debug_assertions) && std::env::var("TRANCE_DEV_PLUGINS").ok().as_deref() != Some("1") {
        return Vec::new();
    }
    let Ok(home) = std::env::var("HOME") else {
        return Vec::new();
    };
    let projects = PathBuf::from(home).join("Projects");
    // Prefer crateria/ layout; keep ubermetroid/ for local checkouts during the rebrand.
    let plugin_roots = [
        projects.join("crateria").join("trance-plugins"),
        projects.join("ubermetroid").join("trance-plugins"),
    ];
    let mut dirs = Vec::new();
    for root in plugin_roots {
        dirs.push(root.join("target").join("release"));
        dirs.push(root.join("target").join("debug"));
        dirs.push(root.join(clean).join("target").join("release"));
        dirs.push(root.join(clean).join("target").join("debug"));
    }
    dirs.push(
        projects
            .join(format!("trance-plugin-{clean}"))
            .join("target")
            .join("release"),
    );
    dirs.push(
        projects
            .join(format!("trance-plugin-{clean}"))
            .join("target")
            .join("debug"),
    );
    dirs
}

pub use crate::launcher_trust::is_trusted_plugin_path;
use crate::launcher_trust::is_trusted_plugin_path_cached;

fn trusted_plugin_dirs(clean: &str, mode: &LaunchMode) -> Vec<PathBuf> {
    let mut dirs = crate::discovery::get_screensaver_dirs();
    if *mode == LaunchMode::Preview {
        dirs.extend(dev_plugin_dirs(clean));
    }
    dirs
}

/// Resolve a saver name to a trusted plugin library path.
pub fn resolve_saver_binary(name: &str, mode: &LaunchMode) -> std::io::Result<PathBuf> {
    if name.contains('/') || name.contains('\\') {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("saver name must not be a path: {name}"),
        ));
    }
    let clean = sanitize_saver_name(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown or invalid screensaver name: {name}"),
        )
    })?;

    if !ALLOWED_SAVERS.contains(&clean.as_str()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("screensaver '{clean}' is not in the trusted allowlist"),
        ));
    }

    let candidates = [
        format!("libscreensaver_{clean}.so"),
        format!("lib{clean}.so"),
        clean.clone(),
    ];

    let find_in_dir = |base: &Path| -> Option<PathBuf> {
        for candidate in &candidates {
            let path = base.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
        None
    };

    let trusted_dirs = trusted_plugin_dirs(&clean, mode);
    // Canonicalize trust roots once — not per candidate file.
    let canonical_trusted: Vec<PathBuf> = trusted_dirs
        .iter()
        .filter_map(|dir| std::fs::canonicalize(dir).ok())
        .collect();
    let dev_dirs = dev_plugin_dirs(&clean);
    let search_order: Vec<&Path> = if *mode == LaunchMode::Preview {
        trusted_dirs
            .iter()
            .map(|p| p.as_path())
            .chain(dev_dirs.iter().map(|p| p.as_path()))
            .collect()
    } else {
        trusted_dirs.iter().map(|p| p.as_path()).collect()
    };

    for base in search_order {
        if let Some(path) = find_in_dir(base)
            && is_trusted_plugin_path_cached(&path, &canonical_trusted)
        {
            return Ok(path);
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("no trusted plugin found for saver '{clean}' (mode {mode:?})"),
    ))
}

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "launcher_proptest.rs"]
mod proptests;
