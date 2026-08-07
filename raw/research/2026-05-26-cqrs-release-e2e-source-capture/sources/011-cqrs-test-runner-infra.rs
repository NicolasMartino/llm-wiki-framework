use crate::error::{Result, TestRunnerError};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

const HEALTH_ENDPOINTS: [(&str, &str); 5] = [
    ("fullstack", "http://localhost:9081/ready"),
    ("user-api", "http://localhost:9061/ready"),
    ("workout-api", "http://localhost:9062/ready"),
    ("exercise-api", "http://localhost:9063/ready"),
    (
        "keycloak",
        "http://localhost:9051/realms/local/.well-known/openid-configuration",
    ),
];

pub struct InfraGuard {
    _root: std::path::PathBuf,
    keep: bool,
}

impl InfraGuard {
    pub fn start(root: &Path, skip: bool, keep: bool, verbose: bool) -> Result<Self> {
        if skip {
            eprintln!("Using existing E2E infrastructure (--skip-infra)");
            // Verify services are actually up
            Self::health_check_with_retry(5, Duration::from_secs(2))?;
            return Ok(Self {
                _root: root.to_path_buf(),
                keep: true,
            });
        }

        eprintln!("Starting E2E infrastructure (pulumi up)...");
        Self::run_just(root, "_e2e-infra-up", verbose)?;
        eprintln!("Waiting for services to become healthy...");
        Self::run_just(root, "_wait-for-services", verbose)?;
        eprintln!("Running final health verification...");

        // Verify health
        Self::health_check_with_retry(10, Duration::from_secs(3))?;

        Ok(Self {
            _root: root.to_path_buf(),
            keep,
        })
    }

    fn health_check_with_retry(max_attempts: u32, base_delay: Duration) -> Result<()> {
        health_check_with(
            max_attempts,
            base_delay,
            |name, url| match ureq::get(url).timeout(Duration::from_secs(5)).call() {
                Ok(resp) => Ok(resp.status()),
                Err(error) => Err(format!("{} failed: {}", name, error)),
            },
            std::thread::sleep,
        )
    }

    fn run_just(root: &Path, recipe: &str, _verbose: bool) -> Result<()> {
        // Use `status()` so stdout/stderr stream live to the terminal.
        let status = Command::new("just")
            .arg(recipe)
            .current_dir(root)
            .status()?;

        if !status.success() {
            return Err(TestRunnerError::CommandFailed {
                cmd: format!("just {}", recipe),
                code: status.code().unwrap_or(-1),
            });
        }

        Ok(())
    }
}

fn health_check_with<F, S>(
    max_attempts: u32,
    base_delay: Duration,
    mut probe: F,
    mut sleep: S,
) -> Result<()>
where
    F: FnMut(&str, &str) -> std::result::Result<u16, String>,
    S: FnMut(Duration),
{
    for attempt in 1..=max_attempts {
        let mut all_healthy = true;

        for (name, url) in HEALTH_ENDPOINTS {
            match probe(name, url) {
                Ok(200) => {}
                Ok(status) => {
                    all_healthy = false;
                    eprintln!("[{}] {} returned status {}", attempt, name, status);
                }
                Err(error) => {
                    all_healthy = false;
                    eprintln!("[{}] {}", attempt, error);
                }
            }
        }

        if all_healthy {
            return Ok(());
        }

        if attempt < max_attempts {
            sleep(base_delay * attempt);
        }
    }

    Err(TestRunnerError::HealthCheckFailed(
        "Services did not become healthy".to_string(),
    ))
}

impl Drop for InfraGuard {
    fn drop(&mut self) {
        if !self.keep {
            eprintln!("Tearing down E2E infrastructure...");
            match Command::new("just")
                .arg("_e2e-infra-down")
                .current_dir(&self._root)
                .status()
            {
                Ok(status) if !status.success() => eprintln!(
                    "Failed to tear down E2E infrastructure (exit code {})",
                    status.code().unwrap_or(-1)
                ),
                Ok(_) => {}
                Err(e) => eprintln!("Failed to run just _e2e-infra-down: {}", e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn health_check_with_succeeds_when_all_endpoints_are_healthy() {
        let calls = Arc::new(Mutex::new(0usize));
        let result = health_check_with(
            3,
            Duration::from_secs(2),
            {
                let calls = Arc::clone(&calls);
                move |_name, _url| {
                    *calls.lock().expect("calls lock") += 1;
                    Ok(200)
                }
            },
            |_| {},
        );

        assert!(result.is_ok());
        assert_eq!(
            *calls.lock().expect("calls lock"),
            HEALTH_ENDPOINTS.len(),
            "one healthy attempt should probe each endpoint exactly once"
        );
    }

    #[test]
    fn health_check_with_retries_with_linear_backoff_until_healthy() {
        let calls = Arc::new(Mutex::new(0usize));
        let sleeps = Arc::new(Mutex::new(Vec::<Duration>::new()));

        let result = health_check_with(
            3,
            Duration::from_secs(2),
            {
                let calls = Arc::clone(&calls);
                move |_name, _url| {
                    let mut calls = calls.lock().expect("calls lock");
                    *calls += 1;
                    if *calls <= HEALTH_ENDPOINTS.len() {
                        Ok(503)
                    } else {
                        Ok(200)
                    }
                }
            },
            {
                let sleeps = Arc::clone(&sleeps);
                move |duration| sleeps.lock().expect("sleeps lock").push(duration)
            },
        );

        assert!(result.is_ok());
        assert_eq!(
            sleeps.lock().expect("sleeps lock").as_slice(),
            &[Duration::from_secs(2)]
        );
    }

    #[test]
    fn health_check_with_returns_failure_after_exhausting_attempts() {
        let sleeps = Arc::new(Mutex::new(Vec::<Duration>::new()));

        let err = health_check_with(
            3,
            Duration::from_secs(1),
            |_name, _url| Err("probe failed".to_string()),
            {
                let sleeps = Arc::clone(&sleeps);
                move |duration| sleeps.lock().expect("sleeps lock").push(duration)
            },
        )
        .expect_err("persistent failures should bubble up");

        assert!(
            err.to_string().contains("Services did not become healthy"),
            "unexpected error: {err}"
        );
        assert_eq!(
            sleeps.lock().expect("sleeps lock").as_slice(),
            &[Duration::from_secs(1), Duration::from_secs(2)]
        );
    }
}
