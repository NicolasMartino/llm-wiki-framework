use crate::cli::Category;
use crate::env::e2e_env_vars;
use crate::error::Result;
use crate::suite::{run_nextest_with_junit, RunContext, SuiteResult, TestSuite};
use std::path::PathBuf;
use std::process::Command;

pub struct E2eSuite;
const E2E_JOBS: &str = "4";

fn e2e_junit_path(ctx: &RunContext) -> PathBuf {
    ctx.root.join("target/nextest/ci/junit-e2e.xml")
}

fn build_e2e_command(ctx: &RunContext) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.arg("nextest")
        .arg("run")
        .arg("--profile")
        .arg("ci")
        .arg("-p")
        .arg("e2e-tests")
        .arg("--features")
        .arg("stack-e2e")
        .arg("-j")
        .arg(E2E_JOBS)
        .current_dir(&ctx.root);

    // Set E2E environment variables
    for (key, value) in e2e_env_vars() {
        cmd.env(key, value);
    }
    if ctx.headed {
        cmd.env("HEADED", "1");
    }

    cmd
}

impl TestSuite for E2eSuite {
    fn name(&self) -> &str {
        "e2e"
    }

    fn category(&self) -> Category {
        Category::E2e
    }

    fn needs_infra(&self) -> bool {
        true
    }

    fn run(&self, ctx: &RunContext) -> Result<SuiteResult> {
        let junit_path = e2e_junit_path(ctx);
        let cmd = build_e2e_command(ctx);
        run_nextest_with_junit(cmd, ctx, &junit_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> RunContext {
        RunContext {
            root: PathBuf::from("/tmp/workspace"),
            filter: vec!["sync_flow".to_string()],
            fail_fast: false,
            headed: true,
            verbose: false,
        }
    }

    #[test]
    fn e2e_uses_isolated_junit_path() {
        let ctx = test_ctx();
        let path = e2e_junit_path(&ctx);
        assert!(path
            .to_string_lossy()
            .ends_with("target/nextest/ci/junit-e2e.xml"));
    }

    #[test]
    fn e2e_command_uses_ci_runner_slots() {
        let ctx = test_ctx();
        let cmd = build_e2e_command(&ctx);
        let args: Vec<String> = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert!(
            args.windows(2)
                .any(|pair| pair[0].as_str() == "-j" && pair[1].as_str() == E2E_JOBS),
            "expected -j {} in command args: {:?}",
            E2E_JOBS,
            args
        );
        assert!(
            args.windows(2)
                .any(|pair| pair[0].as_str() == "--features" && pair[1].as_str() == "stack-e2e"),
            "expected --features stack-e2e in command args: {:?}",
            args
        );
        assert!(
            !args.iter().any(|arg| arg == "sync_flow"),
            "filters are appended by run_nextest_with_junit to avoid duplicate nextest args: {:?}",
            args
        );
    }

    #[test]
    fn e2e_command_sets_expected_environment() {
        let ctx = test_ctx();
        let cmd = build_e2e_command(&ctx);
        let envs: Vec<(String, String)> = cmd
            .get_envs()
            .filter_map(|(key, value)| {
                Some((
                    key.to_string_lossy().to_string(),
                    value?.to_string_lossy().to_string(),
                ))
            })
            .collect();

        assert!(
            envs.iter()
                .any(|(key, value)| key == "HEADED" && value == "1"),
            "expected HEADED=1 in envs: {:?}",
            envs
        );
        assert!(
            envs.iter()
                .any(|(key, value)| key == "BFF_URL" && value == "http://localhost:9081"),
            "expected BFF_URL override in envs: {:?}",
            envs
        );
        assert!(
            envs.iter()
                .any(|(key, value)| key == "KEYCLOAK_URL" && value == "http://localhost:9051"),
            "expected KEYCLOAK_URL override in envs: {:?}",
            envs
        );
    }
}
