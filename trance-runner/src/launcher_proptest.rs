//! Property tests for plugin name sanitization and allowlist policy.

use super::{ALLOWED_SAVERS, is_allowed_saver, sanitize_saver_name};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn sanitize_never_returns_path_separators(s in ".*") {
        if let Some(clean) = sanitize_saver_name(&s) {
            prop_assert!(!clean.contains('/'));
            prop_assert!(!clean.contains('\\'));
            prop_assert!(clean.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
        }
    }

    #[test]
    fn allowlist_members_are_allowed(idx in 0usize..10) {
        let name = ALLOWED_SAVERS[idx % ALLOWED_SAVERS.len()];
        prop_assert!(is_allowed_saver(name));
        let cleaned = sanitize_saver_name(name);
        prop_assert_eq!(cleaned.as_deref(), Some(name));
    }

    #[test]
    fn path_like_names_not_allowed(s in "[a-z]{1,8}") {
        let parent = format!("../{s}");
        let abs = format!("/tmp/{s}");
        prop_assert!(!is_allowed_saver(&parent));
        prop_assert!(!is_allowed_saver(&abs));
    }

    #[test]
    fn libscreensaver_prefix_strips(s in prop::sample::select(vec![
        "beams", "storm", "radar", "hearth"
    ])) {
        let raw = format!("libscreensaver_{s}.so");
        let cleaned = sanitize_saver_name(&raw);
        prop_assert_eq!(cleaned.as_deref(), Some(s));
    }
}
