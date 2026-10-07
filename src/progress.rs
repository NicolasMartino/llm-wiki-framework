use std::fmt;
use std::io::{self, IsTerminal};
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};

const TTY_REFRESH_HZ: u8 = 10;

pub struct ProgressReporter {
    output: ProgressOutput,
}

enum ProgressOutput {
    Terminal,
    Lines,
    #[cfg(test)]
    Hidden,
}

impl ProgressReporter {
    pub fn stderr() -> Self {
        Self {
            output: if io::stderr().is_terminal() {
                ProgressOutput::Terminal
            } else {
                ProgressOutput::Lines
            },
        }
    }

    #[cfg(test)]
    pub fn hidden() -> Self {
        Self {
            output: ProgressOutput::Hidden,
        }
    }

    pub fn begin(
        &mut self,
        phase: &'static str,
        label: &str,
        index: usize,
        total_models: usize,
        total_bytes: u64,
    ) -> ProgressOperation {
        let prefix = format!("[{index}/{total_models}] {label}");
        match self.output {
            ProgressOutput::Terminal => {
                let style = ProgressStyle::with_template(
                    "{prefix} {msg:<8} {percent:>3}%  {bytes} / {total_bytes}  {bytes_per_sec:>10}  eta {floored_eta}",
                )
                .expect("static progress template is valid")
                .with_key("floored_eta", |state: &ProgressState, out: &mut dyn fmt::Write| {
                    let _ = out.write_str(&floored_eta(state.elapsed(), state.fraction(), state.eta()));
                });
                // Style, prefix and message go on before the draw target, so the
                // first frame drawn already names the step.
                let bar = ProgressBar::hidden()
                    .with_style(style)
                    .with_prefix(prefix)
                    .with_message(phase);
                bar.set_length(total_bytes);
                bar.set_draw_target(ProgressDrawTarget::stderr_with_hz(TTY_REFRESH_HZ));
                ProgressOperation::Terminal(bar)
            }
            ProgressOutput::Lines => {
                eprintln!("{prefix} {phase} start {}", format_bytes(total_bytes));
                ProgressOperation::Lines(LineProgress {
                    prefix,
                    phase,
                    total_bytes,
                    transferred: 0,
                    next_percent: 25,
                    started: Instant::now(),
                })
            }
            #[cfg(test)]
            ProgressOutput::Hidden => ProgressOperation::Hidden,
        }
    }
}

pub enum ProgressOperation {
    Terminal(ProgressBar),
    Lines(LineProgress),
    #[cfg(test)]
    Hidden,
}

impl ProgressOperation {
    pub fn advance(&mut self, bytes_delta: u64) {
        match self {
            Self::Terminal(bar) => bar.inc(bytes_delta),
            Self::Lines(lines) => {
                for line in lines.advance_lines(bytes_delta) {
                    eprintln!("{line}");
                }
            }
            #[cfg(test)]
            Self::Hidden => {}
        }
    }

    /// Ends the operation without a done line, for a step that failed.
    pub fn abandon(self) {
        match self {
            Self::Terminal(bar) => bar.abandon(),
            Self::Lines(_) => {}
            #[cfg(test)]
            Self::Hidden => {}
        }
    }

    pub fn finish(self) {
        match self {
            Self::Terminal(bar) => bar.finish(),
            Self::Lines(lines) => eprintln!("{}", lines.finish_line()),
            #[cfg(test)]
            Self::Hidden => {}
        }
    }
}

pub struct LineProgress {
    prefix: String,
    phase: &'static str,
    total_bytes: u64,
    transferred: u64,
    next_percent: u64,
    started: Instant,
}

impl LineProgress {
    fn advance_lines(&mut self, bytes_delta: u64) -> Vec<String> {
        self.transferred = self.transferred.saturating_add(bytes_delta);
        let current_percent = percent(self.transferred, self.total_bytes);
        let mut lines = Vec::new();
        while self.next_percent <= 100 && current_percent >= self.next_percent {
            lines.push(format!(
                "{} {} {}% {} / {} {} eta {}",
                self.prefix,
                self.phase,
                self.next_percent,
                format_bytes(self.transferred),
                format_bytes(self.total_bytes),
                format_rate(self.transferred, self.started.elapsed()),
                format_eta(self.transferred, self.total_bytes, self.started.elapsed())
            ));
            self.next_percent += 25;
        }
        lines
    }

    fn finish_line(self) -> String {
        let elapsed = self.started.elapsed();
        format!(
            "{} {} done {} in {} ({})",
            self.prefix,
            self.phase,
            format_bytes(self.transferred),
            format_duration(elapsed),
            format_rate(self.transferred, elapsed)
        )
    }
}

fn percent(bytes: u64, total: u64) -> u64 {
    if total == 0 {
        100
    } else {
        bytes
            .saturating_mul(100)
            .checked_div(total)
            .unwrap_or(100)
            .min(100)
    }
}

fn format_rate(bytes: u64, elapsed: Duration) -> String {
    let per_second = if elapsed.is_zero() {
        0
    } else {
        (bytes as f64 / elapsed.as_secs_f64()) as u64
    };
    format!("{}/s", format_bytes(per_second))
}

fn eta_is_premature(elapsed: Duration, fraction: f32) -> bool {
    elapsed < Duration::from_secs(2) || fraction < 0.01
}

fn floored_eta(elapsed: Duration, fraction: f32, eta: Duration) -> String {
    if eta_is_premature(elapsed, fraction) {
        "--:--".to_string()
    } else {
        format_seconds(eta.as_secs())
    }
}

fn format_eta(bytes: u64, total: u64, elapsed: Duration) -> String {
    let fraction = if total == 0 {
        1.0
    } else {
        bytes as f32 / total as f32
    };
    if eta_is_premature(elapsed, fraction) || bytes == 0 {
        return "--:--".to_string();
    }
    let seconds =
        (total.saturating_sub(bytes) as f64 / (bytes as f64 / elapsed.as_secs_f64())).ceil() as u64;
    format_seconds(seconds)
}

fn format_duration(duration: Duration) -> String {
    format_seconds(duration.as_secs())
}

fn format_seconds(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.1} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KiB", bytes / KIB)
    } else {
        format!("{} B", bytes as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_byte_boundaries() {
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1024), "1.0 KiB");
        assert_eq!(format_bytes(1023 * 1024 * 1024), "1023.0 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GiB");
    }

    #[test]
    fn eta_is_suppressed_until_two_seconds() {
        assert_eq!(format_eta(50, 100, Duration::from_secs(1)), "--:--");
        assert_eq!(format_eta(50, 100, Duration::from_secs(5)), "00:05");
    }

    #[test]
    fn eta_is_suppressed_below_one_percent() {
        assert_eq!(format_eta(5, 1000, Duration::from_secs(10)), "--:--");
        assert_eq!(
            floored_eta(Duration::from_secs(10), 0.005, Duration::from_secs(9000)),
            "--:--"
        );
        assert_eq!(
            floored_eta(Duration::from_secs(10), 0.5, Duration::from_secs(10)),
            "00:10"
        );
        assert_eq!(
            floored_eta(Duration::from_secs(1), 0.5, Duration::from_secs(1)),
            "--:--"
        );
    }

    #[test]
    fn hidden_reporter_is_a_no_op() {
        let mut reporter = ProgressReporter::hidden();
        let mut operation = reporter.begin("verify", "fixture", 1, 1, 100);
        operation.advance(100);
        operation.finish();
    }

    #[test]
    fn non_tty_output_is_bounded_and_contains_no_terminal_controls() {
        let mut progress = LineProgress {
            prefix: "[1/1] fixture".to_string(),
            phase: "download",
            total_bytes: 100,
            transferred: 0,
            next_percent: 25,
            started: Instant::now(),
        };
        let mut lines = vec!["[1/1] fixture download start 100 B".to_string()];
        for _ in 0..100 {
            lines.extend(progress.advance_lines(1));
        }
        lines.push(progress.finish_line());

        assert!(lines.len() <= 6);
        assert!(
            lines
                .iter()
                .all(|line| !line.contains("\x1b[") && !line.contains('\r'))
        );
        assert!(lines.iter().any(|line| line.contains("download 100%")));
    }
}
