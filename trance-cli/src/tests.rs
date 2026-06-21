// SPDX-License-Identifier: MIT

#[cfg(test)]
mod tests {
    use crate::run;

    #[test]
    fn test_completion_bash() {
        let res = run(vec!["completion".to_string(), "bash".to_string()]);
        assert!(res.is_ok());
    }

    #[test]
    fn test_completion_zsh() {
        let res = run(vec!["completion".to_string(), "zsh".to_string()]);
        assert!(res.is_ok());
    }

    #[test]
    fn test_completion_invalid() {
        let res = run(vec!["completion".to_string(), "invalid".to_string()]);
        assert!(res.is_err());
    }

    #[test]
    fn test_bug_report() {
        let res = run(vec!["bug-report".to_string()]);
        assert!(res.is_ok());
    }

    #[test]
    fn test_self_update() {
        let res = run(vec!["self-update".to_string()]);
        assert!(res.is_ok());
    }

    #[test]
    fn test_clean_stale() {
        let res = run(vec!["clean".to_string()]);
        assert!(res.is_ok());
    }

    #[test]
    fn test_invalid_command() {
        let res = run(vec!["invalid-command-name".to_string()]);
        assert!(res.is_err());
    }
}
