use std::path::Path;
use std::process::Command;

use crate::cli::CliContext;

pub fn exclude_rebuildable_from_time_machine(path: &Path, context: &CliContext) {
    if !should_attempt_time_machine_exclusion(path) {
        context.diagnostic(format!(
            "Time Machine exclusion skipped: {}",
            path.display()
        ));
        return;
    }

    match Command::new("/usr/bin/tmutil")
        .arg("addexclusion")
        .arg(path)
        .status()
    {
        Ok(status) if status.success() => {
            context.diagnostic(format!("Time Machine exclusion added: {}", path.display()));
        }
        Ok(status) => {
            context.diagnostic(format!(
                "Time Machine exclusion failed for {}: exit status {}",
                path.display(),
                status
            ));
        }
        Err(error) => {
            context.diagnostic(format!(
                "Time Machine exclusion unavailable for {}: {}",
                path.display(),
                error
            ));
        }
    }
}

fn should_attempt_time_machine_exclusion(path: &Path) -> bool {
    cfg!(target_os = "macos")
        && std::env::var_os("LLM_WIKI_DISABLE_TIME_MACHINE_EXCLUSION").is_none()
        && path.starts_with("/Users")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::should_attempt_time_machine_exclusion;

    #[test]
    fn time_machine_exclusion_is_limited_to_macos_user_home_paths() {
        let expected = cfg!(target_os = "macos")
            && std::env::var_os("LLM_WIKI_DISABLE_TIME_MACHINE_EXCLUSION").is_none();
        assert_eq!(
            should_attempt_time_machine_exclusion(Path::new("/Users/test/.llm_wiki/models")),
            expected
        );
        assert!(!should_attempt_time_machine_exclusion(Path::new(
            "/tmp/.llm_wiki/models"
        )));
    }
}
