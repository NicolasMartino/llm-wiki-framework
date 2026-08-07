use std::env;
use std::error::Error;
use std::fmt;
#[cfg(unix)]
use std::fs::File;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(unix)]
use std::sync::{Mutex, OnceLock};

use anyhow::Result;

const TEST_RUNTIME_FAILURE_ENV: &str = "LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE";
const RUNTIME_BACKEND_ENV: &str = "LLM_WIKI_GGUF_RUNTIME";
const RUNTIME_STDERR_ENV: &str = "LLM_WIKI_GGUF_RUNTIME_STDERR";
static VERBOSE_STDERR: AtomicBool = AtomicBool::new(false);

pub fn set_verbose_stderr(enabled: bool) {
    VERBOSE_STDERR.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GgufRuntimeBackend {
    Auto,
    Cpu,
}

impl GgufRuntimeBackend {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cpu => "cpu",
        }
    }

    const fn qmd_options(self) -> qmd::RuntimeOptions {
        match self {
            Self::Auto => qmd::RuntimeOptions::auto(),
            Self::Cpu => qmd::RuntimeOptions::cpu(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GgufRuntimeReport {
    requested_backend: GgufRuntimeBackend,
    used_backend: GgufRuntimeBackend,
    fallback: bool,
}

impl GgufRuntimeReport {
    fn new(
        requested_backend: GgufRuntimeBackend,
        used_backend: GgufRuntimeBackend,
        fallback: bool,
    ) -> Self {
        Self {
            requested_backend,
            used_backend,
            fallback,
        }
    }

    pub(crate) fn new_for_probe(
        requested_backend: GgufRuntimeBackend,
        used_backend: GgufRuntimeBackend,
        fallback: bool,
    ) -> Self {
        Self::new(requested_backend, used_backend, fallback)
    }

    pub const fn requested_backend(&self) -> GgufRuntimeBackend {
        self.requested_backend
    }

    pub const fn used_backend(&self) -> GgufRuntimeBackend {
        self.used_backend
    }

    pub const fn fallback(&self) -> bool {
        self.fallback
    }

    pub fn merged(&self, other: &Self) -> Self {
        let used_backend = if self.used_backend == GgufRuntimeBackend::Cpu
            || other.used_backend == GgufRuntimeBackend::Cpu
        {
            GgufRuntimeBackend::Cpu
        } else {
            GgufRuntimeBackend::Auto
        };
        Self {
            requested_backend: self.requested_backend,
            used_backend,
            fallback: self.fallback || other.fallback,
        }
    }
}

pub struct GgufRuntimeValue<T> {
    pub value: T,
    pub report: GgufRuntimeReport,
}

pub struct GgufEmbeddingEngine {
    engine: qmd::EmbeddingEngine,
    model_path: PathBuf,
    requested_backend: GgufRuntimeBackend,
    used_backend: GgufRuntimeBackend,
    fallback: bool,
}

pub struct GgufGenerationEngine {
    engine: qmd::GenerationEngine,
    model_path: PathBuf,
    requested_backend: GgufRuntimeBackend,
    used_backend: GgufRuntimeBackend,
    fallback: bool,
}

pub struct GgufRerankEngine {
    engine: qmd::RerankEngine,
    model_path: PathBuf,
    requested_backend: GgufRuntimeBackend,
    used_backend: GgufRuntimeBackend,
    fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GgufRuntimeRole {
    Embedding,
    QueryExpansion,
    Rerank,
}

impl GgufRuntimeRole {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Embedding => "embedding",
            Self::QueryExpansion => "query_expansion",
            Self::Rerank => "rerank",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GgufRuntimeStage {
    ContextCreate,
    Embedding,
    QueryExpansion,
    Rerank,
}

impl GgufRuntimeStage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ContextCreate => "context_create",
            Self::Embedding => "embedding",
            Self::QueryExpansion => "query_expansion",
            Self::Rerank => "rerank",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GgufRuntimeErrorKind {
    BackendUnavailable,
    ContextCreationFailed,
    InferenceFailed,
}

impl GgufRuntimeErrorKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::BackendUnavailable => "backend_unavailable",
            Self::ContextCreationFailed => "context_creation_failed",
            Self::InferenceFailed => "inference_failed",
        }
    }

    pub const fn readiness_reason(self) -> &'static str {
        match self {
            Self::BackendUnavailable => "runtime_backend_unavailable",
            Self::ContextCreationFailed | Self::InferenceFailed => "runtime_backend_failed",
        }
    }

    const fn is_cpu_retryable(self) -> bool {
        matches!(self, Self::BackendUnavailable | Self::ContextCreationFailed)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GgufRuntimeError {
    role: GgufRuntimeRole,
    stage: GgufRuntimeStage,
    kind: GgufRuntimeErrorKind,
    model_path: PathBuf,
    message: String,
}

impl GgufRuntimeError {
    fn new(
        role: GgufRuntimeRole,
        stage: GgufRuntimeStage,
        model_path: &Path,
        message: String,
    ) -> Self {
        Self {
            role,
            stage,
            kind: classify_runtime_error(&message, stage),
            model_path: model_path.to_path_buf(),
            message,
        }
    }

    fn forced(role: GgufRuntimeRole, stage: GgufRuntimeStage, model_path: &Path) -> Self {
        Self {
            role,
            stage,
            kind: GgufRuntimeErrorKind::BackendUnavailable,
            model_path: model_path.to_path_buf(),
            message: "forced GGUF runtime failure: failed to create command queue".to_string(),
        }
    }

    pub const fn role(&self) -> GgufRuntimeRole {
        self.role
    }

    pub const fn stage(&self) -> GgufRuntimeStage {
        self.stage
    }

    pub const fn kind(&self) -> GgufRuntimeErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GgufRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "GGUF runtime {} failed during {} for {}: {}",
            self.role.label(),
            self.stage.label(),
            self.model_path.display(),
            self.message
        )
    }
}

impl Error for GgufRuntimeError {}

pub fn embedding_engine(model_path: &Path) -> Result<GgufEmbeddingEngine> {
    let requested_backend = requested_backend();
    let (engine, used_backend, fallback) =
        construct_with_cpu_fallback(GgufRuntimeRole::Embedding, model_path, |backend| {
            qmd::EmbeddingEngine::with_runtime_options(model_path, backend.qmd_options())
        })?;
    Ok(GgufEmbeddingEngine {
        engine,
        model_path: model_path.to_path_buf(),
        requested_backend,
        used_backend,
        fallback: requested_backend == GgufRuntimeBackend::Auto && fallback,
    })
}

pub fn embed_document(
    engine: &mut GgufEmbeddingEngine,
    text: &str,
    title: Option<&str>,
) -> Result<GgufRuntimeValue<Vec<f32>>> {
    if let Some(error) = forced_runtime_failure(
        GgufRuntimeRole::Embedding,
        GgufRuntimeStage::Embedding,
        engine.used_backend,
        &engine.model_path,
    ) {
        return Err(error.into());
    }
    match with_runtime_stderr(|| engine.engine.embed_document(text, title)) {
        Ok(embedding) => Ok(GgufRuntimeValue {
            value: embedding.embedding,
            report: engine.report(),
        }),
        Err(error) if engine.can_retry_cpu(&error.to_string(), GgufRuntimeStage::Embedding) => {
            engine.switch_to_cpu()?;
            let embedding = with_runtime_stderr(|| engine.engine.embed_document(text, title))
                .map_err(|error| {
                    GgufRuntimeError::new(
                        GgufRuntimeRole::Embedding,
                        GgufRuntimeStage::Embedding,
                        &engine.model_path,
                        error.to_string(),
                    )
                })?;
            Ok(GgufRuntimeValue {
                value: embedding.embedding,
                report: engine.report(),
            })
        }
        Err(error) => Err(GgufRuntimeError::new(
            GgufRuntimeRole::Embedding,
            GgufRuntimeStage::Embedding,
            &engine.model_path,
            error.to_string(),
        )
        .into()),
    }
}

pub fn embed_query(
    engine: &mut GgufEmbeddingEngine,
    query: &str,
) -> Result<GgufRuntimeValue<Vec<f32>>> {
    if let Some(error) = forced_runtime_failure(
        GgufRuntimeRole::Embedding,
        GgufRuntimeStage::Embedding,
        engine.used_backend,
        &engine.model_path,
    ) {
        return Err(error.into());
    }
    match with_runtime_stderr(|| engine.engine.embed_query(query)) {
        Ok(embedding) => Ok(GgufRuntimeValue {
            value: embedding.embedding,
            report: engine.report(),
        }),
        Err(error) if engine.can_retry_cpu(&error.to_string(), GgufRuntimeStage::Embedding) => {
            engine.switch_to_cpu()?;
            let embedding =
                with_runtime_stderr(|| engine.engine.embed_query(query)).map_err(|error| {
                    GgufRuntimeError::new(
                        GgufRuntimeRole::Embedding,
                        GgufRuntimeStage::Embedding,
                        &engine.model_path,
                        error.to_string(),
                    )
                })?;
            Ok(GgufRuntimeValue {
                value: embedding.embedding,
                report: engine.report(),
            })
        }
        Err(error) => Err(GgufRuntimeError::new(
            GgufRuntimeRole::Embedding,
            GgufRuntimeStage::Embedding,
            &engine.model_path,
            error.to_string(),
        )
        .into()),
    }
}

pub fn generation_engine(model_path: &Path) -> Result<GgufGenerationEngine> {
    let requested_backend = requested_backend();
    let (engine, used_backend, fallback) =
        construct_with_cpu_fallback(GgufRuntimeRole::QueryExpansion, model_path, |backend| {
            qmd::GenerationEngine::with_runtime_options(model_path, backend.qmd_options())
        })?;
    Ok(GgufGenerationEngine {
        engine,
        model_path: model_path.to_path_buf(),
        requested_backend,
        used_backend,
        fallback: requested_backend == GgufRuntimeBackend::Auto && fallback,
    })
}

pub fn expand_query(
    engine: &mut GgufGenerationEngine,
    query: &str,
) -> Result<GgufRuntimeValue<Vec<qmd::Queryable>>> {
    if let Some(error) = forced_runtime_failure(
        GgufRuntimeRole::QueryExpansion,
        GgufRuntimeStage::QueryExpansion,
        engine.used_backend,
        &engine.model_path,
    ) {
        return Err(error.into());
    }
    match with_runtime_stderr(|| engine.engine.expand_query(query, true)) {
        Ok(expanded) => Ok(GgufRuntimeValue {
            value: expanded,
            report: engine.report(),
        }),
        Err(error)
            if engine.can_retry_cpu(&error.to_string(), GgufRuntimeStage::QueryExpansion) =>
        {
            engine.switch_to_cpu()?;
            let expanded = with_runtime_stderr(|| engine.engine.expand_query(query, true))
                .map_err(|error| {
                    GgufRuntimeError::new(
                        GgufRuntimeRole::QueryExpansion,
                        GgufRuntimeStage::QueryExpansion,
                        &engine.model_path,
                        error.to_string(),
                    )
                })?;
            Ok(GgufRuntimeValue {
                value: expanded,
                report: engine.report(),
            })
        }
        Err(error) => Err(GgufRuntimeError::new(
            GgufRuntimeRole::QueryExpansion,
            GgufRuntimeStage::QueryExpansion,
            &engine.model_path,
            error.to_string(),
        )
        .into()),
    }
}

pub fn rerank_engine(model_path: &Path) -> Result<GgufRerankEngine> {
    let requested_backend = requested_backend();
    let (engine, used_backend, fallback) =
        construct_with_cpu_fallback(GgufRuntimeRole::Rerank, model_path, |backend| {
            qmd::RerankEngine::with_runtime_options(model_path, backend.qmd_options())
        })?;
    Ok(GgufRerankEngine {
        engine,
        model_path: model_path.to_path_buf(),
        requested_backend,
        used_backend,
        fallback: requested_backend == GgufRuntimeBackend::Auto && fallback,
    })
}

pub fn rerank(
    engine: &mut GgufRerankEngine,
    query: &str,
    documents: &[qmd::RerankDocument],
) -> Result<GgufRuntimeValue<qmd::BatchRerankResult>> {
    if let Some(error) = forced_runtime_failure(
        GgufRuntimeRole::Rerank,
        GgufRuntimeStage::Rerank,
        engine.used_backend,
        &engine.model_path,
    ) {
        return Err(error.into());
    }
    match with_runtime_stderr(|| engine.engine.rerank(query, documents)) {
        Ok(result) => Ok(GgufRuntimeValue {
            value: result,
            report: engine.report(),
        }),
        Err(error) if engine.can_retry_cpu(&error.to_string(), GgufRuntimeStage::Rerank) => {
            engine.switch_to_cpu()?;
            let result = with_runtime_stderr(|| engine.engine.rerank(query, documents)).map_err(
                |error| {
                    GgufRuntimeError::new(
                        GgufRuntimeRole::Rerank,
                        GgufRuntimeStage::Rerank,
                        &engine.model_path,
                        error.to_string(),
                    )
                },
            )?;
            Ok(GgufRuntimeValue {
                value: result,
                report: engine.report(),
            })
        }
        Err(error) => Err(GgufRuntimeError::new(
            GgufRuntimeRole::Rerank,
            GgufRuntimeStage::Rerank,
            &engine.model_path,
            error.to_string(),
        )
        .into()),
    }
}

impl GgufEmbeddingEngine {
    pub fn report(&self) -> GgufRuntimeReport {
        GgufRuntimeReport::new(self.requested_backend, self.used_backend, self.fallback)
    }

    fn can_retry_cpu(&self, message: &str, stage: GgufRuntimeStage) -> bool {
        self.requested_backend == GgufRuntimeBackend::Auto
            && self.used_backend == GgufRuntimeBackend::Auto
            && is_cpu_retryable(message, stage)
    }

    fn switch_to_cpu(&mut self) -> Result<()> {
        reload_embedding_engine_for_cpu(&mut self.engine, &self.model_path)?;
        self.used_backend = GgufRuntimeBackend::Cpu;
        self.fallback = true;
        Ok(())
    }
}

impl GgufGenerationEngine {
    pub fn report(&self) -> GgufRuntimeReport {
        GgufRuntimeReport::new(self.requested_backend, self.used_backend, self.fallback)
    }

    fn can_retry_cpu(&self, message: &str, stage: GgufRuntimeStage) -> bool {
        self.requested_backend == GgufRuntimeBackend::Auto
            && self.used_backend == GgufRuntimeBackend::Auto
            && is_cpu_retryable(message, stage)
    }

    fn switch_to_cpu(&mut self) -> Result<()> {
        reload_generation_engine_for_cpu(&mut self.engine, &self.model_path)?;
        self.used_backend = GgufRuntimeBackend::Cpu;
        self.fallback = true;
        Ok(())
    }
}

impl GgufRerankEngine {
    pub fn report(&self) -> GgufRuntimeReport {
        GgufRuntimeReport::new(self.requested_backend, self.used_backend, self.fallback)
    }

    fn can_retry_cpu(&self, message: &str, stage: GgufRuntimeStage) -> bool {
        self.requested_backend == GgufRuntimeBackend::Auto
            && self.used_backend == GgufRuntimeBackend::Auto
            && is_cpu_retryable(message, stage)
    }

    fn switch_to_cpu(&mut self) -> Result<()> {
        reload_rerank_engine_for_cpu(&mut self.engine, &self.model_path)?;
        self.used_backend = GgufRuntimeBackend::Cpu;
        self.fallback = true;
        Ok(())
    }
}

fn forced_runtime_failure(
    role: GgufRuntimeRole,
    stage: GgufRuntimeStage,
    backend: GgufRuntimeBackend,
    model_path: &Path,
) -> Option<GgufRuntimeError> {
    let value = env::var(TEST_RUNTIME_FAILURE_ENV).ok()?;
    let value = value.trim();
    let role_label = role.label();
    let stage_label = stage.label();
    let backend_label = backend.label();
    let combined = format!("{role_label}:{stage_label}");
    let backend_combined = format!("{combined}:{backend_label}");
    let role_backend_combined = format!("{role_label}:{backend_label}");
    let matches = value == "1"
        || value == "all"
        || value == role_label
        || value == stage_label
        || value == combined
        || value == backend_combined
        || value == role_backend_combined;
    matches.then(|| GgufRuntimeError::forced(role, stage, model_path))
}

fn construct_with_cpu_fallback<T>(
    role: GgufRuntimeRole,
    model_path: &Path,
    construct: impl Fn(GgufRuntimeBackend) -> Result<T>,
) -> Result<(T, GgufRuntimeBackend, bool)> {
    let requested = requested_backend();
    match requested {
        GgufRuntimeBackend::Cpu => {
            construct_backend(role, GgufRuntimeBackend::Cpu, model_path, construct)
                .map(|engine| (engine, GgufRuntimeBackend::Cpu, false))
        }
        GgufRuntimeBackend::Auto => {
            match construct_backend(role, GgufRuntimeBackend::Auto, model_path, &construct) {
                Ok(engine) => Ok((engine, GgufRuntimeBackend::Auto, false)),
                Err(error) if is_cpu_retryable_error(&error) => {
                    construct_backend(role, GgufRuntimeBackend::Cpu, model_path, construct)
                        .map(|engine| (engine, GgufRuntimeBackend::Cpu, true))
                }
                Err(error) => Err(error),
            }
        }
    }
}

fn construct_backend<T>(
    role: GgufRuntimeRole,
    backend: GgufRuntimeBackend,
    model_path: &Path,
    construct: impl Fn(GgufRuntimeBackend) -> Result<T>,
) -> Result<T> {
    if let Some(error) =
        forced_runtime_failure(role, GgufRuntimeStage::ContextCreate, backend, model_path)
    {
        return Err(error.into());
    }
    with_runtime_stderr(|| construct(backend)).map_err(|error| {
        GgufRuntimeError::new(
            role,
            GgufRuntimeStage::ContextCreate,
            model_path,
            error.to_string(),
        )
        .into()
    })
}

fn reload_embedding_engine_for_cpu(
    engine: &mut qmd::EmbeddingEngine,
    model_path: &Path,
) -> Result<()> {
    with_runtime_stderr(|| {
        engine.reload_with_runtime_options(model_path, GgufRuntimeBackend::Cpu.qmd_options())
    })
    .map_err(|error| {
        GgufRuntimeError::new(
            GgufRuntimeRole::Embedding,
            GgufRuntimeStage::ContextCreate,
            model_path,
            error.to_string(),
        )
        .into()
    })
}

fn reload_generation_engine_for_cpu(
    engine: &mut qmd::GenerationEngine,
    model_path: &Path,
) -> Result<()> {
    with_runtime_stderr(|| {
        engine.reload_with_runtime_options(model_path, GgufRuntimeBackend::Cpu.qmd_options())
    })
    .map_err(|error| {
        GgufRuntimeError::new(
            GgufRuntimeRole::QueryExpansion,
            GgufRuntimeStage::ContextCreate,
            model_path,
            error.to_string(),
        )
        .into()
    })
}

fn reload_rerank_engine_for_cpu(engine: &mut qmd::RerankEngine, model_path: &Path) -> Result<()> {
    with_runtime_stderr(|| {
        engine.reload_with_runtime_options(model_path, GgufRuntimeBackend::Cpu.qmd_options())
    })
    .map_err(|error| {
        GgufRuntimeError::new(
            GgufRuntimeRole::Rerank,
            GgufRuntimeStage::ContextCreate,
            model_path,
            error.to_string(),
        )
        .into()
    })
}

pub fn requested_backend() -> GgufRuntimeBackend {
    env::var(RUNTIME_BACKEND_ENV)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| value == "cpu")
        .map(|_| GgufRuntimeBackend::Cpu)
        .unwrap_or(GgufRuntimeBackend::Auto)
}

fn with_runtime_stderr<T>(operation: impl FnOnce() -> T) -> T {
    if runtime_stderr_verbose() {
        return operation();
    }
    with_quiet_stderr(operation)
}

fn runtime_stderr_verbose() -> bool {
    VERBOSE_STDERR.load(Ordering::Relaxed)
        || env::var(RUNTIME_STDERR_ENV)
            .ok()
            .is_some_and(|value| matches!(value.trim(), "1" | "true" | "verbose" | "inherit"))
}

#[cfg(unix)]
fn with_quiet_stderr<T>(operation: impl FnOnce() -> T) -> T {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _lock = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _guard = StderrRedirect::new().ok();
    operation()
}

#[cfg(not(unix))]
fn with_quiet_stderr<T>(operation: impl FnOnce() -> T) -> T {
    operation()
}

#[cfg(unix)]
struct StderrRedirect {
    saved_fd: i32,
}

#[cfg(unix)]
impl StderrRedirect {
    fn new() -> std::io::Result<Self> {
        let dev_null = File::options().write(true).open("/dev/null")?;
        let saved_fd = unsafe { dup(STDERR_FILENO) };
        if saved_fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if unsafe { dup2(dev_null.as_raw_fd(), STDERR_FILENO) } < 0 {
            let error = std::io::Error::last_os_error();
            unsafe {
                close(saved_fd);
            }
            return Err(error);
        }
        Ok(Self { saved_fd })
    }
}

#[cfg(unix)]
impl Drop for StderrRedirect {
    fn drop(&mut self) {
        unsafe {
            dup2(self.saved_fd, STDERR_FILENO);
            close(self.saved_fd);
        }
    }
}

#[cfg(unix)]
const STDERR_FILENO: i32 = 2;

#[cfg(unix)]
unsafe extern "C" {
    fn dup(fd: i32) -> i32;
    fn dup2(fd: i32, fd2: i32) -> i32;
    fn close(fd: i32) -> i32;
}

fn is_cpu_retryable_error(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<GgufRuntimeError>()
        .is_some_and(|error| error.kind().is_cpu_retryable())
}

fn is_cpu_retryable(message: &str, stage: GgufRuntimeStage) -> bool {
    classify_runtime_error(message, stage).is_cpu_retryable()
}

fn classify_runtime_error(message: &str, stage: GgufRuntimeStage) -> GgufRuntimeErrorKind {
    let normalized = message.to_ascii_lowercase();
    if normalized.contains("failed to create command queue")
        || normalized.contains("failed to initialize backend")
        || normalized.contains("failed to initialize  backend")
        || normalized.contains("failed to allocate context")
    {
        return GgufRuntimeErrorKind::BackendUnavailable;
    }
    if normalized.contains("failed to create context")
        || normalized.contains("failed to initialize the context")
        || normalized.contains("null reference from llama.cpp")
    {
        return GgufRuntimeErrorKind::ContextCreationFailed;
    }
    match stage {
        GgufRuntimeStage::ContextCreate => GgufRuntimeErrorKind::ContextCreationFailed,
        GgufRuntimeStage::Embedding
        | GgufRuntimeStage::QueryExpansion
        | GgufRuntimeStage::Rerank => GgufRuntimeErrorKind::InferenceFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::{GgufRuntimeErrorKind, GgufRuntimeStage, classify_runtime_error};

    #[test]
    fn classify_metal_command_queue_failure_as_backend_unavailable() {
        let message = "ggml_metal_init: error: failed to create command queue";

        assert_eq!(
            classify_runtime_error(message, GgufRuntimeStage::ContextCreate),
            GgufRuntimeErrorKind::BackendUnavailable
        );
    }

    #[test]
    fn classify_qmd_context_failure_as_context_creation_failed() {
        let message = "Failed to create context";

        assert_eq!(
            classify_runtime_error(message, GgufRuntimeStage::QueryExpansion),
            GgufRuntimeErrorKind::ContextCreationFailed
        );
    }
}
