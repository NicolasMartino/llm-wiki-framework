use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result, anyhow, bail};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Parser)]
#[command(
    name = "llm-wiki-release-e2e",
    version,
    about = "Release-artifact E2E runner for llm-wiki"
)]
struct Cli {
    #[command(subcommand)]
    command: Lane,
}

#[derive(Debug, Subcommand)]
enum Lane {
    Smoke(RunArgs),
    Search(RunArgs),
    Linux(LinuxArgs),
    Gguf(RunArgs),
    Windows(RunArgs),
    All(RunArgs),
}

#[derive(Clone, Debug, Args)]
struct RunArgs {
    #[arg(long)]
    artifact: PathBuf,
    #[arg(long)]
    checksum: Option<PathBuf>,
    #[arg(long)]
    target_triple: Option<String>,
    #[arg(long, default_value = "target/release-e2e")]
    output_dir: PathBuf,
    #[arg(long)]
    skip_infra: bool,
    #[arg(long)]
    keep_infra: bool,
    #[arg(long, short)]
    verbose: bool,
    #[arg(long)]
    stdout: bool,
}

#[derive(Clone, Debug, Args)]
struct LinuxArgs {
    #[command(flatten)]
    run: RunArgs,
    #[arg(long, default_value = "debian:bookworm-slim")]
    docker_image: String,
    #[arg(long)]
    docker_platform: Option<String>,
    #[arg(long)]
    docker_network: Option<String>,
    #[arg(long)]
    require_native: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let report = match cli.command {
        Lane::Smoke(args) => run_smoke(args)?,
        Lane::Search(args) => run_search(args)?,
        Lane::Linux(args) => run_linux(args)?,
        Lane::Gguf(_) => bail!("release-e2e gguf lane is not implemented yet"),
        Lane::Windows(_) => bail!("release-e2e windows lane is not implemented yet"),
        Lane::All(_) => bail!("release-e2e all lane is not implemented yet"),
    };

    if report.stdout_requested {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "release-e2e {} passed: {}",
            report.lane,
            report.report_path.display()
        );
    }

    Ok(())
}

fn run_smoke(args: RunArgs) -> Result<RunReport> {
    let lane = "smoke";
    let started_at = Utc::now().to_rfc3339();
    let lane_dir = absolute_path(&args.output_dir)?.join(lane);
    prepare_lane_dir(&lane_dir)?;

    let artifact = args
        .artifact
        .canonicalize()
        .with_context(|| format!("resolve artifact path {}", args.artifact.display()))?;
    if !artifact.is_file() {
        bail!("artifact is not a file: {}", artifact.display());
    }

    let artifact_sha256 = sha256_file(&artifact)?;
    let checksum = verify_checksum(args.checksum.as_deref(), &artifact_sha256)?;
    require_checksum_match(&checksum, &artifact_sha256)?;
    let temp_home = TempHome::create(&lane_dir, args.keep_infra)?;
    let stdout_dir = lane_dir.join("stdout");
    let stderr_dir = lane_dir.join("stderr");
    fs::create_dir_all(&stdout_dir).context("create stdout report dir")?;
    fs::create_dir_all(&stderr_dir).context("create stderr report dir")?;

    let version_command = ShellCommand::new(&artifact, ["--version"]);
    let version_result = run_shell_command(
        "version",
        &version_command,
        &temp_home.path,
        None,
        &stdout_dir.join("version.out"),
        &stderr_dir.join("version.err"),
        args.verbose,
    )?;

    let mut assertions = vec![
        AssertionReport::passed("artifact exists"),
        AssertionReport::passed("artifact sha256 computed"),
        AssertionReport::new("version exits successfully", version_result.success),
        AssertionReport::new("isolated home was used", temp_home.path.exists()),
    ];
    if let Some(checksum) = &checksum {
        assertions.push(AssertionReport::new("checksum matches", checksum.matches));
    } else {
        assertions.push(AssertionReport::passed("checksum not provided"));
    }

    let mut report = RunReport {
        schema_version: 1,
        lane: lane.to_string(),
        success: assertions.iter().all(|assertion| assertion.passed),
        started_at,
        finished_at: Utc::now().to_rfc3339(),
        artifact_path: artifact,
        artifact_sha256,
        checksum,
        target_triple: args.target_triple.unwrap_or_else(host_target_label),
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        execution: host_execution_report(),
        output_dir: lane_dir.clone(),
        temp_home: temp_home.path.clone(),
        temp_home_removed: false,
        skip_infra: args.skip_infra,
        keep_infra: args.keep_infra,
        stdout_requested: args.stdout,
        commands: vec![version_result],
        assertions,
        report_path: lane_dir.join("report.json"),
        junit_path: lane_dir.join("junit.xml"),
        state_manifest_path: lane_dir.join("state-manifest.json"),
    };

    if !report.success {
        if !args.keep_infra {
            temp_home.cleanup()?;
            report.temp_home_removed = true;
        }
        write_reports(&report)?;
        bail!(
            "release-e2e smoke failed; report: {}",
            report.report_path.display()
        );
    }

    if !args.keep_infra {
        temp_home.cleanup()?;
        report.temp_home_removed = true;
    }

    write_reports(&report)?;
    Ok(report)
}

fn run_lane_command(
    executable: &Path,
    args: &[&str],
    name: &str,
    home: &Path,
    cwd: &Path,
    stdout_dir: &Path,
    stderr_dir: &Path,
    verbose: bool,
) -> Result<CommandReport> {
    let command = ShellCommand::new(executable, args.iter().copied());
    run_shell_command(
        name,
        &command,
        home,
        Some(cwd),
        &stdout_dir.join(format!("{name}.out")),
        &stderr_dir.join(format!("{name}.err")),
        verbose,
    )
}

#[derive(Clone, Debug)]
enum ProductExecutionConfig {
    Host,
    Docker(DockerConfig),
}

#[derive(Clone, Debug)]
struct DockerConfig {
    image: String,
    platform: String,
    network: String,
    architecture_native: bool,
    proof_kind: String,
}

struct ProductStory {
    mode: ProductExecutionConfig,
    artifact_exec: PathBuf,
    managed_binary_host: PathBuf,
    managed_binary_exec: PathBuf,
    home_exec: PathBuf,
    run_dir_exec: PathBuf,
    project_dir_exec: PathBuf,
    build_dir_exec: PathBuf,
    artifact_host: PathBuf,
    home_host: PathBuf,
    run_dir_host: PathBuf,
    stdout_dir: PathBuf,
    stderr_dir: PathBuf,
    verbose: bool,
}

impl ProductStory {
    fn host(
        artifact: &Path,
        home: &Path,
        run_dir: &Path,
        stdout_dir: &Path,
        stderr_dir: &Path,
        verbose: bool,
    ) -> Self {
        Self {
            mode: ProductExecutionConfig::Host,
            artifact_exec: artifact.to_path_buf(),
            managed_binary_host: home.join(".llm_wiki").join("bin").join(llm_wiki_exe_name()),
            managed_binary_exec: home.join(".llm_wiki").join("bin").join(llm_wiki_exe_name()),
            home_exec: home.to_path_buf(),
            run_dir_exec: run_dir.to_path_buf(),
            project_dir_exec: run_dir.join("probe project"),
            build_dir_exec: run_dir.join("build-output"),
            artifact_host: artifact.to_path_buf(),
            home_host: home.to_path_buf(),
            run_dir_host: run_dir.to_path_buf(),
            stdout_dir: stdout_dir.to_path_buf(),
            stderr_dir: stderr_dir.to_path_buf(),
            verbose,
        }
    }

    fn docker(
        config: DockerConfig,
        artifact: &Path,
        home: &Path,
        run_dir: &Path,
        stdout_dir: &Path,
        stderr_dir: &Path,
        verbose: bool,
    ) -> Self {
        let home_exec = PathBuf::from("/home/e2e");
        let run_dir_exec = PathBuf::from("/work/run");
        Self {
            mode: ProductExecutionConfig::Docker(config),
            artifact_exec: PathBuf::from("/artifact/llm-wiki"),
            managed_binary_host: home.join(".llm_wiki").join("bin").join("llm-wiki"),
            managed_binary_exec: home_exec.join(".llm_wiki").join("bin").join("llm-wiki"),
            home_exec: home_exec.clone(),
            run_dir_exec: run_dir_exec.clone(),
            project_dir_exec: run_dir_exec.join("probe project"),
            build_dir_exec: run_dir_exec.join("build-output"),
            artifact_host: artifact.to_path_buf(),
            home_host: home.to_path_buf(),
            run_dir_host: run_dir.to_path_buf(),
            stdout_dir: stdout_dir.to_path_buf(),
            stderr_dir: stderr_dir.to_path_buf(),
            verbose,
        }
    }

    fn run(&self, executable: &Path, args: &[&str], name: &str) -> Result<CommandReport> {
        match &self.mode {
            ProductExecutionConfig::Host => run_lane_command(
                executable,
                args,
                name,
                &self.home_exec,
                &self.run_dir_exec,
                &self.stdout_dir,
                &self.stderr_dir,
                self.verbose,
            ),
            ProductExecutionConfig::Docker(config) => run_docker_lane_command(
                config,
                executable,
                args,
                name,
                self,
                &self.stdout_dir.join(format!("{name}.out")),
                &self.stderr_dir.join(format!("{name}.err")),
            ),
        }
    }

    fn run_artifact(&self, args: &[&str], name: &str) -> Result<CommandReport> {
        self.run(&self.artifact_exec, args, name)
    }

    fn run_managed(&self, args: &[&str], name: &str) -> Result<CommandReport> {
        self.run(&self.managed_binary_exec, args, name)
    }

    fn execution_report(&self) -> ExecutionReport {
        match &self.mode {
            ProductExecutionConfig::Host => ExecutionReport {
                kind: "host".to_string(),
                proof_kind: "host_current_platform".to_string(),
                docker_image: None,
                docker_platform: None,
                docker_network: None,
                architecture_native: None,
            },
            ProductExecutionConfig::Docker(config) => ExecutionReport {
                kind: "docker".to_string(),
                proof_kind: config.proof_kind.clone(),
                docker_image: Some(config.image.clone()),
                docker_platform: Some(config.platform.clone()),
                docker_network: Some(config.network.clone()),
                architecture_native: Some(config.architecture_native),
            },
        }
    }
}

fn run_search(args: RunArgs) -> Result<RunReport> {
    run_product_story(args, "search", ProductExecutionConfig::Host)
}

fn run_linux(args: LinuxArgs) -> Result<RunReport> {
    ensure_linux_artifact(&args.run.artifact)?;
    let platform = args
        .docker_platform
        .clone()
        .unwrap_or_else(default_linux_platform);
    let architecture_native = docker_platform_arch(&platform)
        .zip(host_docker_arch())
        .is_some_and(|(container, host)| container == host);
    let proof_kind = linux_proof_kind(&platform);
    if args.require_native && proof_kind != "native_linux_container" {
        bail!(
            "linux lane is {proof_kind}, not native; rerun without --require-native or use a native Linux runner"
        );
    }
    let network = args.docker_network.clone().unwrap_or_else(|| {
        if args.run.skip_infra {
            "none".to_string()
        } else {
            default_docker_network_name()
        }
    });
    let docker = DockerConfig {
        image: args.docker_image.clone(),
        platform,
        network,
        architecture_native,
        proof_kind,
    };
    run_product_story(args.run, "linux", ProductExecutionConfig::Docker(docker))
}

fn run_product_story(
    args: RunArgs,
    lane: &'static str,
    execution_config: ProductExecutionConfig,
) -> Result<RunReport> {
    let started_at = Utc::now().to_rfc3339();
    let lane_dir = absolute_path(&args.output_dir)?.join(lane);
    prepare_lane_dir(&lane_dir)?;

    let artifact = args
        .artifact
        .canonicalize()
        .with_context(|| format!("resolve artifact path {}", args.artifact.display()))?;
    if !artifact.is_file() {
        bail!("artifact is not a file: {}", artifact.display());
    }

    let artifact_sha256 = sha256_file(&artifact)?;
    let checksum = verify_checksum(args.checksum.as_deref(), &artifact_sha256)?;
    require_checksum_match(&checksum, &artifact_sha256)?;
    let temp_home = TempHome::create(&lane_dir, args.keep_infra)?;
    let state_dir = lane_dir.join("state");
    let run_dir = state_dir.join("run");
    let project_dir = run_dir.join("probe project");
    let build_dir = run_dir.join("build-output");
    let stdout_dir = lane_dir.join("stdout");
    let stderr_dir = lane_dir.join("stderr");
    fs::create_dir_all(&run_dir).context("create isolated run dir")?;
    fs::create_dir_all(&stdout_dir).context("create stdout report dir")?;
    fs::create_dir_all(&stderr_dir).context("create stderr report dir")?;
    let _docker_network = DockerNetworkGuard::start(
        &execution_config,
        args.skip_infra,
        args.keep_infra,
        args.verbose,
    )?;
    let story = match execution_config {
        ProductExecutionConfig::Host => ProductStory::host(
            &artifact,
            &temp_home.path,
            &run_dir,
            &stdout_dir,
            &stderr_dir,
            args.verbose,
        ),
        ProductExecutionConfig::Docker(config) => ProductStory::docker(
            config,
            &artifact,
            &temp_home.path,
            &run_dir,
            &stdout_dir,
            &stderr_dir,
            args.verbose,
        ),
    };
    let execution = story.execution_report();

    let managed_binary = story.managed_binary_host.clone();
    let manifest_path = temp_home.path.join(".llm_wiki").join("manifest.json");
    let search_config_path = temp_home.path.join(".llm_wiki").join("search.toml");
    let registry_path = temp_home
        .path
        .join(".local")
        .join("share")
        .join("llm-wiki")
        .join("projects.json");
    let project_id = "release-e2e-probe";
    let index_dir = temp_home
        .path
        .join(".llm_wiki")
        .join("indexes")
        .join(project_id);
    let qmd_store = index_dir.join("qmd-rs.sqlite");

    let mut commands = Vec::new();
    let mut assertions = vec![
        AssertionReport::passed("artifact exists"),
        AssertionReport::passed("artifact sha256 computed"),
    ];
    if let Some(checksum) = &checksum {
        assertions.push(AssertionReport::new("checksum matches", checksum.matches));
    } else {
        assertions.push(AssertionReport::passed("checksum not provided"));
    }

    let version = story.run_artifact(&["--version"], "version")?;
    assertions.push(AssertionReport::new(
        "version exits successfully",
        version.success,
    ));
    commands.push(version);

    let install = story.run_artifact(
        &[
            "install",
            "--non-interactive",
            "--disable-llm-search",
            "--skip-path-guidance",
        ],
        "install",
    )?;
    assertions.push(AssertionReport::new(
        "install exits successfully",
        install.success,
    ));
    commands.push(install);
    assertions.extend([
        file_exists_assertion("managed binary exists", &managed_binary),
        file_exists_assertion("install manifest exists", &manifest_path),
        file_contains_assertion(
            "search config records disabled search",
            &search_config_path,
            "llm_search_enabled = false",
        ),
        file_exists_assertion(
            "claude wiki-init skill exists",
            &temp_home.path.join(".claude/skills/wiki-init/SKILL.md"),
        ),
        file_exists_assertion(
            "codex wiki-query skill exists",
            &temp_home.path.join(".codex/skills/wiki-query/SKILL.md"),
        ),
        file_exists_assertion(
            "codex wiki dispatcher skill exists",
            &temp_home.path.join(".codex/skills/wiki/SKILL.md"),
        ),
    ]);

    let managed_version = story.run_managed(&["--version"], "managed-version")?;
    assertions.push(AssertionReport::new(
        "managed binary exits successfully",
        managed_version.success,
    ));
    commands.push(managed_version);
    let managed_sha_after_install = path_sha256(&managed_binary);

    let second_install = story.run_artifact(
        &[
            "install",
            "--non-interactive",
            "--disable-llm-search",
            "--skip-path-guidance",
        ],
        "install-second",
    )?;
    assertions.push(AssertionReport::new(
        "second install exits successfully",
        second_install.success,
    ));
    commands.push(second_install);
    let managed_sha_after_second_install = path_sha256(&managed_binary);
    assertions.extend([
        file_exists_assertion(
            "managed binary remains after second install",
            &managed_binary,
        ),
        file_exists_assertion(
            "install manifest remains after second install",
            &manifest_path,
        ),
        AssertionReport::new(
            "managed binary hash is stable across second install",
            managed_sha_after_install
                .as_ref()
                .zip(managed_sha_after_second_install.as_ref())
                .is_some_and(|(before, after)| before == after),
        ),
    ]);

    let path_cmd = story.run_artifact(&["path"], "path")?;
    assertions.extend([
        AssertionReport::new("path exits successfully", path_cmd.success),
        stdout_contains_assertion("path reports managed binary", &path_cmd, ".llm_wiki/bin"),
        stdout_contains_assertion("path renders PATH guidance", &path_cmd, "PATH"),
    ]);
    commands.push(path_cmd);

    let status = story.run_artifact(&["status"], "status")?;
    assertions.extend([
        AssertionReport::new("status exits successfully", status.success),
        stdout_contains_assertion(
            "status reports installed version",
            &status,
            "installed version",
        ),
    ]);
    commands.push(status);

    let doctor = story.run_artifact(&["doctor"], "doctor")?;
    assertions.extend([
        AssertionReport::new("doctor exits successfully", doctor.success),
        stdout_contains_assertion(
            "doctor reports disabled LLM search",
            &doctor,
            "LLM search disabled",
        ),
    ]);
    commands.push(doctor);

    let build_out = story.build_dir_exec.display().to_string();
    let build = story.run_artifact(&["build", "--out", &build_out], "build")?;
    assertions.extend([
        AssertionReport::new("build exits successfully", build.success),
        file_exists_assertion(
            "build emits codex wiki-query skill",
            &build_dir.join(".codex/skills/wiki-query/SKILL.md"),
        ),
        file_exists_assertion(
            "build emits claude wiki-init skill",
            &build_dir.join(".claude/skills/wiki-init/SKILL.md"),
        ),
    ]);
    commands.push(build);

    let project_path = story.project_dir_exec.display().to_string();
    let init = story.run_artifact(
        &[
            "init",
            &project_path,
            "--no-register",
            "--non-interactive",
            "--name",
            "Release E2E Probe",
            "--description",
            "Probe project for release E2E",
            "--blueprint",
            "cli-tool",
            "--pack",
            "code",
        ],
        "init",
    )?;
    assertions.extend([
        AssertionReport::new("init exits successfully", init.success),
        file_exists_assertion("init writes AGENTS.md", &project_dir.join("AGENTS.md")),
        file_exists_assertion("init writes wiki index", &project_dir.join("wiki/index.md")),
        file_exists_assertion("init writes wiki log", &project_dir.join("wiki/log.md")),
        file_exists_assertion(
            "init writes project search config",
            &project_dir.join(".llm_wiki/search.toml"),
        ),
    ]);
    commands.push(init);

    let register = story.run_artifact(
        &[
            "register",
            &project_path,
            "--id",
            project_id,
            "--name",
            "Release E2E Probe",
        ],
        "register",
    )?;
    assertions.extend([
        AssertionReport::new("register exits successfully", register.success),
        file_exists_assertion("project registry exists", &registry_path),
        file_contains_assertion(
            "project registry records project id",
            &registry_path,
            project_id,
        ),
    ]);
    commands.push(register);

    let projects_before_index =
        story.run_artifact(&["projects", "--format", "json"], "projects-before-index")?;
    assertions.extend([
        AssertionReport::new(
            "projects before index exits successfully",
            projects_before_index.success,
        ),
        json_pointer_string_assertion(
            "projects before index reports project id",
            &projects_before_index,
            "/projects/0/id",
            project_id,
        ),
        json_pointer_string_assertion(
            "projects before index reports missing index",
            &projects_before_index,
            "/projects/0/index_status",
            "index-missing",
        ),
    ]);
    commands.push(projects_before_index);

    let index = story.run_artifact(&["index", "--project", project_id], "index")?;
    assertions.extend([
        AssertionReport::new("index exits successfully", index.success),
        file_exists_assertion("qmd-rs sqlite store exists", &qmd_store),
    ]);
    commands.push(index);

    let search = story.run_artifact(
        &[
            "search",
            "--mode",
            "lexical",
            "--project",
            project_id,
            "--format",
            "json",
            "Probe",
        ],
        "search-lexical",
    )?;
    assertions.extend([
        AssertionReport::new("lexical search exits successfully", search.success),
        json_pointer_string_assertion(
            "lexical search selects lexical mode",
            &search,
            "/selected_mode",
            "lexical",
        ),
        json_array_non_empty_assertion("lexical search returns results", &search, "/results"),
        json_pointer_string_assertion(
            "lexical search backend is ready",
            &search,
            "/backend_status/state",
            "ready",
        ),
    ]);
    commands.push(search);

    let search_all = story.run_artifact(
        &[
            "search-all",
            "--mode",
            "lexical",
            "--format",
            "json",
            "Probe",
        ],
        "search-all-lexical",
    )?;
    assertions.extend([
        AssertionReport::new("lexical search-all exits successfully", search_all.success),
        json_pointer_string_assertion(
            "search-all selects lexical mode",
            &search_all,
            "/selected_mode",
            "lexical",
        ),
        json_array_non_empty_assertion("search-all returns results", &search_all, "/results"),
        json_pointer_string_assertion(
            "search-all reports project id",
            &search_all,
            "/projects/0/project_id",
            project_id,
        ),
    ]);
    commands.push(search_all);

    let semantic = story.run_artifact(
        &[
            "search",
            "--mode",
            "semantic",
            "--project",
            project_id,
            "--format",
            "json",
            "Probe",
        ],
        "search-semantic-fail-closed",
    )?;
    assertions.extend([
        AssertionReport::new("semantic search fails closed", !semantic.success),
        json_pointer_string_assertion(
            "semantic fail-closed reason is disabled search",
            &semantic,
            "/readiness_reason",
            "llm_search_disabled",
        ),
    ]);
    commands.push(semantic);

    let hybrid = story.run_artifact(
        &[
            "search",
            "--mode",
            "hybrid",
            "--project",
            project_id,
            "--format",
            "json",
            "Probe",
        ],
        "search-hybrid-fail-closed",
    )?;
    assertions.extend([
        AssertionReport::new("hybrid search fails closed", !hybrid.success),
        json_pointer_string_assertion(
            "hybrid fail-closed reason is disabled search",
            &hybrid,
            "/readiness_reason",
            "llm_search_disabled",
        ),
    ]);
    commands.push(hybrid);

    let forget = story.run_artifact(
        &["forget", project_id, "--delete-cache"],
        "forget-delete-cache",
    )?;
    assertions.extend([
        AssertionReport::new("forget exits successfully", forget.success),
        path_missing_assertion("forget removes qmd-rs cache directory", &index_dir),
    ]);
    commands.push(forget);

    let projects_after_forget =
        story.run_artifact(&["projects", "--format", "json"], "projects-after-forget")?;
    assertions.extend([
        AssertionReport::new(
            "projects after forget exits successfully",
            projects_after_forget.success,
        ),
        json_array_empty_assertion(
            "projects after forget is empty",
            &projects_after_forget,
            "/projects",
        ),
    ]);
    commands.push(projects_after_forget);

    let uninstall = story.run_artifact(&["uninstall", "--include-binary"], "uninstall")?;
    assertions.extend([
        AssertionReport::new("uninstall exits successfully", uninstall.success),
        path_missing_assertion("uninstall removes managed binary", &managed_binary),
        path_missing_assertion("uninstall removes manifest", &manifest_path),
        path_missing_assertion(
            "uninstall removes codex wiki-query skill",
            &temp_home.path.join(".codex/skills/wiki-query/SKILL.md"),
        ),
        path_missing_assertion(
            "uninstall removes claude wiki-init skill",
            &temp_home.path.join(".claude/skills/wiki-init/SKILL.md"),
        ),
    ]);
    commands.push(uninstall);

    let mut report = RunReport {
        schema_version: 1,
        lane: lane.to_string(),
        success: assertions.iter().all(|assertion| assertion.passed),
        started_at,
        finished_at: Utc::now().to_rfc3339(),
        artifact_path: artifact,
        artifact_sha256,
        checksum,
        target_triple: args
            .target_triple
            .unwrap_or_else(|| default_target_triple_for_execution(&execution)),
        host_os: std::env::consts::OS.to_string(),
        host_arch: std::env::consts::ARCH.to_string(),
        execution,
        output_dir: lane_dir.clone(),
        temp_home: temp_home.path.clone(),
        temp_home_removed: false,
        skip_infra: args.skip_infra,
        keep_infra: args.keep_infra,
        stdout_requested: args.stdout,
        commands,
        assertions,
        report_path: lane_dir.join("report.json"),
        junit_path: lane_dir.join("junit.xml"),
        state_manifest_path: lane_dir.join("state-manifest.json"),
    };

    if !report.success {
        if !args.keep_infra {
            temp_home.cleanup()?;
            report.temp_home_removed = true;
        }
        write_reports(&report)?;
        bail!(
            "release-e2e {lane} failed; report: {}",
            report.report_path.display()
        );
    }

    if !args.keep_infra {
        temp_home.cleanup()?;
        report.temp_home_removed = true;
    }

    write_reports(&report)?;
    Ok(report)
}

fn prepare_lane_dir(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)
            .with_context(|| format!("remove previous lane dir {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("create lane dir {}", path.display()))
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()
            .context("resolve current directory")?
            .join(path))
    }
}

fn ensure_linux_artifact(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if bytes.starts_with(b"\x7fELF") {
        Ok(())
    } else {
        bail!(
            "linux lane requires an unpacked Linux ELF llm-wiki artifact, got {}",
            path.display()
        )
    }
}

fn llm_wiki_exe_name() -> &'static str {
    if cfg!(windows) {
        "llm-wiki.exe"
    } else {
        "llm-wiki"
    }
}

fn path_sha256(path: &Path) -> Option<String> {
    sha256_file(path).ok()
}

fn file_exists_assertion(name: impl Into<String>, path: &Path) -> AssertionReport {
    AssertionReport::new(name, path.is_file())
}

fn path_missing_assertion(name: impl Into<String>, path: &Path) -> AssertionReport {
    AssertionReport::new(name, !path.exists())
}

fn file_contains_assertion(name: impl Into<String>, path: &Path, needle: &str) -> AssertionReport {
    AssertionReport::new(
        name,
        fs::read_to_string(path).is_ok_and(|content| content.contains(needle)),
    )
}

fn stdout_contains_assertion(
    name: impl Into<String>,
    command: &CommandReport,
    needle: &str,
) -> AssertionReport {
    file_contains_assertion(name, &command.stdout_path, needle)
}

fn json_pointer_string_assertion(
    name: impl Into<String>,
    command: &CommandReport,
    pointer: &str,
    expected: &str,
) -> AssertionReport {
    AssertionReport::new(
        name,
        command_stdout_json(command)
            .ok()
            .and_then(|json| {
                json.pointer(pointer)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .is_some_and(|actual| actual == expected),
    )
}

fn json_array_non_empty_assertion(
    name: impl Into<String>,
    command: &CommandReport,
    pointer: &str,
) -> AssertionReport {
    AssertionReport::new(
        name,
        command_stdout_json(command)
            .ok()
            .and_then(|json| json.pointer(pointer).and_then(Value::as_array).cloned())
            .is_some_and(|items| !items.is_empty()),
    )
}

fn json_array_empty_assertion(
    name: impl Into<String>,
    command: &CommandReport,
    pointer: &str,
) -> AssertionReport {
    AssertionReport::new(
        name,
        command_stdout_json(command)
            .ok()
            .and_then(|json| json.pointer(pointer).and_then(Value::as_array).cloned())
            .is_some_and(|items| items.is_empty()),
    )
}

fn command_stdout_json(command: &CommandReport) -> Result<Value> {
    let stdout = fs::read_to_string(&command.stdout_path)
        .with_context(|| format!("read {}", command.stdout_path.display()))?;
    serde_json::from_str(&stdout).with_context(|| format!("parse JSON from {}", command.name))
}

fn verify_checksum(path: Option<&Path>, actual_sha256: &str) -> Result<Option<ChecksumReport>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let expected = read_checksum(path)?;
    Ok(Some(ChecksumReport {
        path: path.to_path_buf(),
        expected_sha256: expected.clone(),
        matches: expected.eq_ignore_ascii_case(actual_sha256),
    }))
}

fn require_checksum_match(checksum: &Option<ChecksumReport>, actual_sha256: &str) -> Result<()> {
    let Some(checksum) = checksum else {
        return Ok(());
    };
    if checksum.matches {
        return Ok(());
    }
    bail!(
        "checksum mismatch for {}: expected {}, actual {}",
        checksum.path.display(),
        checksum.expected_sha256,
        actual_sha256
    )
}

fn read_checksum(path: &Path) -> Result<String> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("read checksum file {}", path.display()))?;
    content
        .split_whitespace()
        .find(|token| token.len() == 64 && token.chars().all(|ch| ch.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| {
            anyhow!(
                "checksum file {} does not contain a SHA-256",
                path.display()
            )
        })
}

fn sha256_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let digest = Sha256::digest(bytes);
    Ok(format!("{digest:x}"))
}

fn run_shell_command(
    name: &str,
    command: &ShellCommand,
    home: &Path,
    cwd: Option<&Path>,
    stdout_path: &Path,
    stderr_path: &Path,
    verbose: bool,
) -> Result<CommandReport> {
    if verbose {
        eprintln!("running: {}", command.display);
    }

    let stdout_file = fs::File::create(stdout_path)
        .with_context(|| format!("create {}", stdout_path.display()))?;
    let stderr_file = fs::File::create(stderr_path)
        .with_context(|| format!("create {}", stderr_path.display()))?;

    let mut process = Command::new(&command.program);
    if let Some(cwd) = cwd {
        process.current_dir(cwd);
    }
    process
        .args(&command.args)
        .env("HOME", home)
        .env("XDG_CACHE_HOME", home.join(".cache"))
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env_remove("LLM_WIKI_GGUF_RUNTIME")
        .env_remove("LLM_WIKI_TEST_EMBEDDINGS")
        .env_remove("LLM_WIKI_TEST_QUERY_EXPANSION")
        .env_remove("LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE")
        .env_remove("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE")
        .env_remove("LLM_WIKI_TEST_INDEX_SLEEP_MS")
        .env_remove("LLM_WIKI_TEST_MODEL_DOWNLOADS")
        .env_remove("LLM_WIKI_TEST_PROMOTE_MARKER")
        .env_remove("LLM_WIKI_TEST_PROMOTE_PRE_COMMIT_SLEEP_MS")
        .env_remove("LLM_WIKI_TEST_RERANK")
        .env_remove("LLM_WIKI_TEST_REGISTRY_WRITE_DELAY_MS")
        .env_remove("LLM_WIKI_TEST_SEARCH_AFTER_STATUS_SLEEP_MS")
        .env_remove("RUST_LOG")
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));

    let status = process.status().with_context(|| format!("run {name}"))?;
    Ok(CommandReport {
        name: name.to_string(),
        display: command.display.clone(),
        status_code: status.code(),
        success: status.success(),
        stdout_path: stdout_path.to_path_buf(),
        stderr_path: stderr_path.to_path_buf(),
    })
}

fn run_docker_lane_command(
    config: &DockerConfig,
    executable: &Path,
    args: &[&str],
    name: &str,
    story: &ProductStory,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<CommandReport> {
    let inner_command = posix_command_display(executable, args);
    let display = format!(
        "docker run --rm --platform {} --network {} {} sh -lc {}",
        config.platform,
        config.network,
        config.image,
        posix_shell_quote(&inner_command)
    );
    if story.verbose {
        eprintln!("running: {display}");
    }

    let stdout_file = fs::File::create(stdout_path)
        .with_context(|| format!("create {}", stdout_path.display()))?;
    let stderr_file = fs::File::create(stderr_path)
        .with_context(|| format!("create {}", stderr_path.display()))?;

    let artifact_mount = format!("{}:/artifact/llm-wiki:ro", story.artifact_host.display());
    let home_mount = format!("{}:/home/e2e", story.home_host.display());
    let run_mount = format!("{}:/work/run", story.run_dir_host.display());
    let container_name = format!(
        "llm-wiki-release-e2e-{}-{}",
        name.replace('_', "-"),
        std::process::id()
    );

    let status = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("--name")
        .arg(container_name)
        .arg("--platform")
        .arg(&config.platform)
        .arg("--network")
        .arg(&config.network)
        .arg("-v")
        .arg(artifact_mount)
        .arg("-v")
        .arg(home_mount)
        .arg("-v")
        .arg(run_mount)
        .arg("-w")
        .arg("/work/run")
        .arg("-e")
        .arg("HOME=/home/e2e")
        .arg("-e")
        .arg("XDG_CACHE_HOME=/home/e2e/.cache")
        .arg("-e")
        .arg("XDG_CONFIG_HOME=/home/e2e/.config")
        .arg("-e")
        .arg("XDG_DATA_HOME=/home/e2e/.local/share")
        .arg(&config.image)
        .arg("sh")
        .arg("-lc")
        .arg(&inner_command)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .status()
        .with_context(|| format!("run docker command {name}"))?;

    Ok(CommandReport {
        name: name.to_string(),
        display,
        status_code: status.code(),
        success: status.success(),
        stdout_path: stdout_path.to_path_buf(),
        stderr_path: stderr_path.to_path_buf(),
    })
}

fn posix_command_display(executable: &Path, args: &[&str]) -> String {
    let mut command = posix_shell_quote(&executable.display().to_string());
    for arg in args {
        command.push(' ');
        command.push_str(&posix_shell_quote(arg));
    }
    command
}

struct DockerNetworkGuard {
    name: String,
    keep: bool,
    active: bool,
}

impl DockerNetworkGuard {
    fn start(
        config: &ProductExecutionConfig,
        skip_infra: bool,
        keep_infra: bool,
        verbose: bool,
    ) -> Result<Option<Self>> {
        let ProductExecutionConfig::Docker(config) = config else {
            return Ok(None);
        };
        if skip_infra || matches!(config.network.as_str(), "none" | "host" | "bridge") {
            return Ok(Some(Self {
                name: config.network.clone(),
                keep: true,
                active: false,
            }));
        }

        if verbose {
            eprintln!("creating docker network: {}", config.network);
        }
        let status = Command::new("docker")
            .arg("network")
            .arg("create")
            .arg("--internal")
            .arg(&config.network)
            .status()
            .with_context(|| format!("create docker network {}", config.network))?;
        if !status.success() {
            bail!(
                "docker network create failed for {} with exit code {}",
                config.network,
                status.code().unwrap_or(-1)
            );
        }
        Ok(Some(Self {
            name: config.network.clone(),
            keep: keep_infra,
            active: true,
        }))
    }
}

impl Drop for DockerNetworkGuard {
    fn drop(&mut self) {
        if self.active && !self.keep {
            let _ = Command::new("docker")
                .arg("network")
                .arg("rm")
                .arg(&self.name)
                .status();
        }
    }
}

fn write_reports(report: &RunReport) -> Result<()> {
    fs::write(&report.report_path, serde_json::to_string_pretty(report)?)
        .with_context(|| format!("write {}", report.report_path.display()))?;
    fs::write(
        &report.state_manifest_path,
        serde_json::to_string_pretty(&StateManifest::from_report(report))?,
    )
    .with_context(|| format!("write {}", report.state_manifest_path.display()))?;
    fs::write(&report.junit_path, junit_xml(report))
        .with_context(|| format!("write {}", report.junit_path.display()))?;
    Ok(())
}

fn junit_xml(report: &RunReport) -> String {
    let failures = usize::from(!report.success);
    let failure_xml = if report.success {
        String::new()
    } else {
        format!(
            "<failure message=\"release-e2e {} failed\"/>",
            xml_escape(&report.lane)
        )
    };
    format!(
        "<testsuite name=\"llm-wiki-release-e2e-{lane}\" tests=\"1\" failures=\"{failures}\"><testcase name=\"{lane}\">{failure_xml}</testcase></testsuite>\n",
        lane = xml_escape(&report.lane),
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn host_target_label() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

fn host_execution_report() -> ExecutionReport {
    ExecutionReport {
        kind: "host".to_string(),
        proof_kind: "host_current_platform".to_string(),
        docker_image: None,
        docker_platform: None,
        docker_network: None,
        architecture_native: None,
    }
}

fn default_target_triple_for_execution(execution: &ExecutionReport) -> String {
    execution
        .docker_platform
        .as_deref()
        .and_then(linux_target_triple_for_platform)
        .unwrap_or_else(host_target_label)
}

fn default_linux_platform() -> String {
    match std::env::consts::ARCH {
        "aarch64" => "linux/arm64".to_string(),
        "x86_64" => "linux/amd64".to_string(),
        arch => format!("linux/{arch}"),
    }
}

fn default_docker_network_name() -> String {
    format!("llm-wiki-release-e2e-{}", std::process::id())
}

fn docker_platform_arch(platform: &str) -> Option<&'static str> {
    if platform.ends_with("/arm64") {
        Some("arm64")
    } else if platform.ends_with("/amd64") {
        Some("amd64")
    } else {
        None
    }
}

fn host_docker_arch() -> Option<&'static str> {
    match std::env::consts::ARCH {
        "aarch64" => Some("arm64"),
        "x86_64" => Some("amd64"),
        _ => None,
    }
}

fn linux_target_triple_for_platform(platform: &str) -> Option<String> {
    if platform.ends_with("/arm64") {
        Some("aarch64-unknown-linux-gnu".to_string())
    } else if platform.ends_with("/amd64") {
        Some("x86_64-unknown-linux-gnu".to_string())
    } else {
        None
    }
}

fn linux_proof_kind(platform: &str) -> String {
    let architecture_native = docker_platform_arch(platform)
        .zip(host_docker_arch())
        .is_some_and(|(container, host)| container == host);
    if std::env::consts::OS == "linux" && architecture_native {
        "native_linux_container".to_string()
    } else if architecture_native {
        "virtualized_linux_container".to_string()
    } else {
        "emulated_linux_container_packaging_smoke".to_string()
    }
}

#[derive(Debug)]
struct TempHome {
    path: PathBuf,
    keep: bool,
}

impl TempHome {
    fn create(lane_dir: &Path, keep: bool) -> Result<Self> {
        let path = lane_dir.join("state").join(format!(
            "home-{}-{}",
            Utc::now().timestamp_millis(),
            std::process::id()
        ));
        fs::create_dir_all(path.join(".cache")).context("create isolated cache dir")?;
        fs::create_dir_all(path.join(".config")).context("create isolated config dir")?;
        fs::create_dir_all(path.join(".local/share")).context("create isolated data dir")?;
        Ok(Self { path, keep })
    }

    fn cleanup(&self) -> Result<()> {
        if !self.keep && self.path.exists() {
            fs::remove_dir_all(&self.path)
                .with_context(|| format!("remove isolated home {}", self.path.display()))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ShellCommand {
    program: OsString,
    args: Vec<OsString>,
    display: String,
}

impl ShellCommand {
    fn new<I, S>(executable: &Path, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let args: Vec<String> = args
            .into_iter()
            .map(|arg| arg.as_ref().to_string())
            .collect();
        #[cfg(windows)]
        {
            let mut command = format!("& {}", powershell_quote(&executable.display().to_string()));
            for arg in &args {
                command.push(' ');
                command.push_str(&powershell_quote(arg));
            }
            Self {
                program: OsString::from("powershell.exe"),
                args: vec![
                    OsString::from("-NoProfile"),
                    OsString::from("-NonInteractive"),
                    OsString::from("-ExecutionPolicy"),
                    OsString::from("Bypass"),
                    OsString::from("-Command"),
                    OsString::from(command.clone()),
                ],
                display: command,
            }
        }
        #[cfg(not(windows))]
        {
            let mut command = posix_shell_quote(&executable.display().to_string());
            for arg in &args {
                command.push(' ');
                command.push_str(&posix_shell_quote(arg));
            }
            Self {
                program: OsString::from("sh"),
                args: vec![OsString::from("-c"), OsString::from(command.clone())],
                display: command,
            }
        }
    }
}

fn posix_shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(windows)]
fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[derive(Debug, Serialize)]
struct RunReport {
    schema_version: u32,
    lane: String,
    success: bool,
    started_at: String,
    finished_at: String,
    artifact_path: PathBuf,
    artifact_sha256: String,
    checksum: Option<ChecksumReport>,
    target_triple: String,
    host_os: String,
    host_arch: String,
    execution: ExecutionReport,
    output_dir: PathBuf,
    temp_home: PathBuf,
    temp_home_removed: bool,
    skip_infra: bool,
    keep_infra: bool,
    stdout_requested: bool,
    commands: Vec<CommandReport>,
    assertions: Vec<AssertionReport>,
    report_path: PathBuf,
    junit_path: PathBuf,
    state_manifest_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct StateManifest<'a> {
    artifact_path: &'a Path,
    artifact_sha256: &'a str,
    target_triple: &'a str,
    host_os: &'a str,
    host_arch: &'a str,
    execution: &'a ExecutionReport,
    temp_home: &'a Path,
    temp_home_removed: bool,
    commands: &'a [CommandReport],
    assertions: &'a [AssertionReport],
}

impl<'a> StateManifest<'a> {
    fn from_report(report: &'a RunReport) -> Self {
        Self {
            artifact_path: &report.artifact_path,
            artifact_sha256: &report.artifact_sha256,
            target_triple: &report.target_triple,
            host_os: &report.host_os,
            host_arch: &report.host_arch,
            execution: &report.execution,
            temp_home: &report.temp_home,
            temp_home_removed: report.temp_home_removed,
            commands: &report.commands,
            assertions: &report.assertions,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct ExecutionReport {
    kind: String,
    proof_kind: String,
    docker_image: Option<String>,
    docker_platform: Option<String>,
    docker_network: Option<String>,
    architecture_native: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ChecksumReport {
    path: PathBuf,
    expected_sha256: String,
    matches: bool,
}

#[derive(Debug, Serialize)]
struct CommandReport {
    name: String,
    display: String,
    status_code: Option<i32>,
    success: bool,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct AssertionReport {
    name: String,
    passed: bool,
}

impl AssertionReport {
    fn passed(name: impl Into<String>) -> Self {
        Self::new(name, true)
    }

    fn new(name: impl Into<String>, passed: bool) -> Self {
        Self {
            name: name.into(),
            passed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_reader_accepts_shasum_format() {
        let dir = tempfile::tempdir().expect("tempdir");
        let checksum = dir.path().join("SHA256SUMS");
        fs::write(
            &checksum,
            "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd  llm-wiki\n",
        )
        .expect("checksum");

        let parsed = read_checksum(&checksum).expect("parsed checksum");
        assert_eq!(
            parsed,
            "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
        );
    }

    #[test]
    fn smoke_lane_rejects_checksum_before_running_artifact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let artifact = dir.path().join("llm-wiki");
        fs::write(&artifact, b"not an executable").expect("artifact");
        let checksum = dir.path().join("SHA256SUMS");
        fs::write(&checksum, format!("{}  llm-wiki\n", "0".repeat(64))).expect("checksum");
        let output_dir = dir.path().join("reports");

        let err = run_smoke(RunArgs {
            artifact,
            checksum: Some(checksum),
            target_triple: None,
            output_dir: output_dir.clone(),
            skip_infra: false,
            keep_infra: false,
            verbose: false,
            stdout: false,
        })
        .expect_err("checksum mismatch should fail before execution");

        assert!(err.to_string().contains("checksum mismatch"));
        assert!(!output_dir.join("smoke/stdout/version.out").exists());
    }

    #[test]
    fn state_manifest_serializes_command_and_assertion_data() {
        let report = RunReport {
            schema_version: 1,
            lane: "smoke".to_string(),
            success: true,
            started_at: "2026-05-26T00:00:00Z".to_string(),
            finished_at: "2026-05-26T00:00:01Z".to_string(),
            artifact_path: PathBuf::from("/tmp/llm-wiki"),
            artifact_sha256: "a".repeat(64),
            checksum: None,
            target_triple: "aarch64-apple-darwin".to_string(),
            host_os: "macos".to_string(),
            host_arch: "aarch64".to_string(),
            execution: host_execution_report(),
            output_dir: PathBuf::from("target/release-e2e/smoke"),
            temp_home: PathBuf::from("target/release-e2e/smoke/state/home"),
            temp_home_removed: true,
            skip_infra: false,
            keep_infra: false,
            stdout_requested: false,
            commands: vec![CommandReport {
                name: "version".to_string(),
                display: "'/tmp/llm-wiki' '--version'".to_string(),
                status_code: Some(0),
                success: true,
                stdout_path: PathBuf::from("stdout/version.out"),
                stderr_path: PathBuf::from("stderr/version.err"),
            }],
            assertions: vec![AssertionReport::passed("version exits successfully")],
            report_path: PathBuf::from("report.json"),
            junit_path: PathBuf::from("junit.xml"),
            state_manifest_path: PathBuf::from("state-manifest.json"),
        };

        let json = serde_json::to_value(StateManifest::from_report(&report)).expect("json");
        assert_eq!(json["artifact_sha256"], "a".repeat(64));
        assert_eq!(json["execution"]["kind"], "host");
        assert_eq!(json["commands"][0]["name"], "version");
        assert_eq!(json["assertions"][0]["passed"], true);
    }

    #[test]
    fn temp_home_cleanup_removes_isolated_home_when_not_kept() {
        let dir = tempfile::tempdir().expect("tempdir");
        let home = TempHome::create(dir.path(), false).expect("home");
        let path = home.path.clone();
        assert!(path.exists());

        home.cleanup().expect("cleanup");
        assert!(!path.exists());
    }

    #[cfg(not(windows))]
    #[test]
    fn shell_command_uses_unix_shell_quoting() {
        let command = ShellCommand::new(Path::new("/tmp/llm wiki's/bin"), ["--version"]);
        assert_eq!(command.program, OsString::from("sh"));
        assert_eq!(command.display, "'/tmp/llm wiki'\\''s/bin' '--version'");
    }

    #[test]
    fn linux_platform_maps_to_release_target_triples() {
        assert_eq!(
            linux_target_triple_for_platform("linux/arm64").as_deref(),
            Some("aarch64-unknown-linux-gnu")
        );
        assert_eq!(
            linux_target_triple_for_platform("linux/amd64").as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
        assert_eq!(linux_target_triple_for_platform("linux/riscv64"), None);
    }

    #[test]
    fn docker_execution_report_records_platform_and_network() {
        let dir = tempfile::tempdir().expect("tempdir");
        let artifact = dir.path().join("llm-wiki");
        fs::write(&artifact, b"\x7fELF").expect("artifact");
        let run_dir = dir.path().join("run");
        let stdout_dir = dir.path().join("stdout");
        let stderr_dir = dir.path().join("stderr");
        let config = DockerConfig {
            image: "debian:bookworm-slim".to_string(),
            platform: "linux/arm64".to_string(),
            network: "llm-wiki-release-e2e-test".to_string(),
            architecture_native: true,
            proof_kind: "native_linux_container".to_string(),
        };

        let story = ProductStory::docker(
            config,
            &artifact,
            dir.path(),
            &run_dir,
            &stdout_dir,
            &stderr_dir,
            false,
        );
        let report = story.execution_report();

        assert_eq!(report.kind, "docker");
        assert_eq!(report.docker_platform.as_deref(), Some("linux/arm64"));
        assert_eq!(
            report.docker_network.as_deref(),
            Some("llm-wiki-release-e2e-test")
        );
        assert_eq!(story.artifact_exec, PathBuf::from("/artifact/llm-wiki"));
        assert_eq!(
            story.managed_binary_exec,
            PathBuf::from("/home/e2e/.llm_wiki/bin/llm-wiki")
        );
    }
}
