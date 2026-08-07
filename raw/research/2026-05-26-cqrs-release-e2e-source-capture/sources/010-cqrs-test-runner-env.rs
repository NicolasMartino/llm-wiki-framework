use std::path::Path;

/// Returns E2E environment variables by loading the test-runner profile.
pub fn e2e_env_vars() -> Vec<(String, String)> {
    let profile = find_profile();
    load_profile(&profile)
}

fn find_profile() -> std::path::PathBuf {
    // Walk upward from CARGO_MANIFEST_DIR to find the repo root (contains config/runtime/)
    let start = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut dir = start;
    loop {
        let candidate = dir.join("config/runtime/e2e/local/test-runner.env");
        if candidate.exists() {
            return candidate;
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => panic!(
                "Could not find config/runtime/e2e/local/test-runner.env starting from {}",
                start.display()
            ),
        }
    }
}

fn load_profile(path: &Path) -> Vec<(String, String)> {
    let content = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let eq = trimmed.find('=')?;
            let key = trimmed[..eq].to_string();
            let value = trimmed[eq + 1..].to_string();
            Some((key, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2e_env_vars_has_required_keys() {
        let vars = e2e_env_vars();
        let keys: Vec<&str> = vars.iter().map(|(k, _)| k.as_str()).collect();

        assert!(keys.contains(&"BFF_URL"));
        assert!(keys.contains(&"APP_URL"));
        assert!(keys.contains(&"KAFKA_BOOTSTRAP_SERVERS"));
        assert!(keys.contains(&"KEYCLOAK_URL"));
        assert!(keys.contains(&"EXERCISE_API_DATABASE_URL"));
    }

    #[test]
    fn test_e2e_env_vars_uses_9xxx_ports() {
        let vars = e2e_env_vars();

        let bff_url = vars
            .iter()
            .find(|(k, _)| k == "BFF_URL")
            .map(|(_, v)| v.as_str());
        assert_eq!(bff_url, Some("http://localhost:9081"));

        let keycloak_url = vars
            .iter()
            .find(|(k, _)| k == "KEYCLOAK_URL")
            .map(|(_, v)| v.as_str());
        assert_eq!(keycloak_url, Some("http://localhost:9051"));

        let kafka = vars
            .iter()
            .find(|(k, _)| k == "KAFKA_BOOTSTRAP_SERVERS")
            .map(|(_, v)| v.as_str());
        assert_eq!(kafka, Some("localhost:19092"));

        let db_url = vars
            .iter()
            .find(|(k, _)| k == "USER_API_DATABASE_URL")
            .map(|(_, v)| v.as_str());
        assert!(db_url.unwrap().contains(":9060"));
    }
}
