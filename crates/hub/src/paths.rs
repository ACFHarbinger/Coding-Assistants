use std::path::PathBuf;

/// Resolve the Hub / settings data directory.
///
/// `CA_HOME` wins when set (used as the directory itself). Otherwise this is
/// `$HOME/.coding-assistants`, or `./.coding-assistants` if `HOME` is unset.
/// The path is not symlink-resolved; callers that key workspace overrides
/// must keep the same rule.
pub fn default_hub_home() -> PathBuf {
    if let Ok(home) = std::env::var("CA_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".coding-assistants")
}

#[cfg(test)]
mod tests {
    use super::default_hub_home;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn ca_home_wins_when_set() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("CA_HOME", "/tmp/ca-home-test");
        assert_eq!(
            default_hub_home(),
            std::path::PathBuf::from("/tmp/ca-home-test")
        );
        std::env::remove_var("CA_HOME");
    }

    #[test]
    fn empty_ca_home_falls_back_to_home_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("CA_HOME", "   ");
        let resolved = default_hub_home();
        std::env::remove_var("CA_HOME");
        assert!(resolved.ends_with(".coding-assistants"));
    }
}
