use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use rmcp::{
    ErrorData, Peer, RoleServer, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, ContentBlock, Implementation, ListResourcesResult, MetaObject,
        PaginatedRequestParams, ReadResourceRequestParams, ReadResourceResponse,
        ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerConfig,
    },
    service::RequestContext,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::Mutex,
};
use tokio_util::sync::CancellationToken;

use crate::{Result, config::Config};

const OUTPUT_PAGE_MAX: usize = 64 * 1024;
const MAX_RUN_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
const MAX_RUN_DIFF_BYTES: usize = 16 * 1024 * 1024;
const COMPLETED_RUN_LIMIT: usize = 32;
const COMPLETED_RUN_TTL: Duration = Duration::from_secs(30 * 60);
const DASHBOARD_URI: &str = "ui://hk/run-dashboard";
const MCP_APP_MIME: &str = "text/html;profile=mcp-app";
const DASHBOARD_HTML: &str = include_str!("mcp_dashboard.html");

fn dashboard_tool_meta() -> MetaObject {
    serde_json::from_value(json!({
        "ui": {"resourceUri": DASHBOARD_URI},
        "openai/outputTemplate": DASHBOARD_URI,
    }))
    .expect("dashboard metadata is an object")
}

/// Run an MCP server for coding agents over standard input/output
#[derive(usage_rs::Args)]
pub struct Mcp {
    /// Restrict hk tools to this project root (defaults to the current directory)
    #[usage(long, value_name = "PATH", value_hint = ValueHint::DirPath)]
    root: Option<PathBuf>,
}

impl Mcp {
    pub async fn run(self) -> Result<()> {
        let root = canonical_directory(self.root.as_deref().unwrap_or(Path::new(".")))?;
        let service = HkMcpServer::new(root)
            .serve(stdio())
            .await
            .map_err(|error| eyre::eyre!("failed to start MCP server: {error}"))?;
        service
            .waiting()
            .await
            .map_err(|error| eyre::eyre!("MCP server failed: {error}"))?;
        Ok(())
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    let root = path
        .canonicalize()
        .map_err(|error| eyre::eyre!("invalid MCP root {}: {error}", path.display()))?;
    if !root.is_dir() {
        return Err(eyre::eyre!(
            "MCP root is not a directory: {}",
            root.display()
        ));
    }
    Ok(root)
}

#[derive(Debug, Clone, Copy)]
enum RunKind {
    Check,
    SafeCheck,
    SafeFix,
}

impl RunKind {
    fn label(self) -> &'static str {
        match self {
            Self::Check => "check",
            Self::SafeCheck => "safe_check",
            Self::SafeFix => "safe_fix",
        }
    }

    fn command(self) -> &'static str {
        match self {
            Self::Check | Self::SafeCheck => "check",
            Self::SafeFix => "fix",
        }
    }

    fn safe(self) -> bool {
        !matches!(self, Self::Check)
    }
}

#[derive(Debug)]
struct RunRecord {
    id: String,
    root: PathBuf,
    kind: RunKind,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    completed_at: Option<Instant>,
    exit_code: Option<i32>,
    output: Vec<u8>,
    stdout: Vec<u8>,
    stdout_event_buffer: Vec<u8>,
    output_truncated: bool,
    stdout_truncated: bool,
    saw_run_completed: bool,
    result: Option<Value>,
    diff: String,
    diff_truncated: bool,
    error: Option<String>,
    cancel: CancellationToken,
}

impl RunRecord {
    fn active(&self) -> bool {
        matches!(self.status.as_str(), "starting" | "running" | "cancelling")
    }

    fn snapshot(&self) -> RunSnapshot {
        RunSnapshot {
            schema_version: 1,
            id: self.id.clone(),
            root: self.root.display().to_string(),
            kind: self.kind.label().to_string(),
            status: self.status.clone(),
            started_at: self.started_at.clone(),
            finished_at: self.finished_at.clone(),
            exit_code: self.exit_code,
            output_bytes: self.output.len(),
            output_truncated: self.output_truncated,
            has_diff: !self.diff.is_empty(),
            diff_bytes: self.diff.len(),
            diff_truncated: self.diff_truncated,
            result: self.result.clone(),
            error: self.error.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
struct RunSnapshot {
    schema_version: u32,
    id: String,
    root: String,
    kind: String,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    exit_code: Option<i32>,
    output_bytes: usize,
    output_truncated: bool,
    has_diff: bool,
    diff_bytes: usize,
    diff_truncated: bool,
    result: Option<Value>,
    error: Option<String>,
}

#[derive(Debug, Default)]
struct McpState {
    startup_roots: BTreeSet<PathBuf>,
    roots: BTreeSet<PathBuf>,
    runs: VecDeque<RunRecord>,
}

impl McpState {
    fn replace_client_roots(&mut self, client_roots: BTreeSet<PathBuf>) {
        self.roots = self.startup_roots.union(&client_roots).cloned().collect();
    }

    fn cleanup(&mut self) {
        self.cleanup_at(Instant::now());
    }

    fn cleanup_at(&mut self, now: Instant) {
        self.runs.retain(|run| {
            run.completed_at
                .is_none_or(|completed| now.duration_since(completed) <= COMPLETED_RUN_TTL)
        });
        let mut completed = self.runs.iter().filter(|run| !run.active()).count();
        while completed > COMPLETED_RUN_LIMIT {
            if let Some(index) = self.runs.iter().position(|run| !run.active()) {
                self.runs.remove(index);
                completed -= 1;
            } else {
                break;
            }
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RootRequest {
    /// An allowed root returned by inspect_project; omit when only one root is available.
    root: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct RunRequest {
    /// Run identifier returned by a start tool.
    run_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct OutputRequest {
    /// Run identifier returned by a start tool.
    run_id: String,
    /// Byte offset at which to start this page.
    #[serde(default)]
    offset: usize,
    /// Requested page size in bytes; capped at 65536.
    limit: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct OutputPage {
    schema_version: u32,
    run_id: String,
    offset: usize,
    next_offset: usize,
    total_bytes: usize,
    eof: bool,
    truncated: bool,
    text: String,
}

#[derive(Debug, Clone)]
struct HkMcpServer {
    state: Arc<Mutex<McpState>>,
    next_id: Arc<AtomicU64>,
    tool_router: ToolRouter<Self>,
}

impl HkMcpServer {
    fn new(root: PathBuf) -> Self {
        let mut roots = BTreeSet::new();
        roots.insert(root);
        Self {
            state: Arc::new(Mutex::new(McpState {
                startup_roots: roots.clone(),
                roots,
                runs: VecDeque::new(),
            })),
            next_id: Arc::new(AtomicU64::new(1)),
            tool_router: Self::combined_tool_router(),
        }
    }

    #[allow(deprecated)]
    async fn refresh_client_roots(&self, peer: &Peer<RoleServer>) {
        let supports_roots = peer
            .peer_info()
            .is_some_and(|info| info.capabilities.roots.is_some());
        if !supports_roots {
            return;
        }
        let result = match peer.list_roots().await {
            Ok(result) => result,
            Err(_) => {
                self.state
                    .lock()
                    .await
                    .replace_client_roots(BTreeSet::new());
                return;
            }
        };
        let mut client_roots = BTreeSet::new();
        {
            let state = self.state.lock().await;
            for root in result.roots {
                let Ok(uri) = url::Url::parse(&root.uri) else {
                    continue;
                };
                let Ok(path) = uri.to_file_path() else {
                    continue;
                };
                if let Ok(path) = canonical_directory(&path)
                    && is_within_startup_roots(&state.startup_roots, &path)
                {
                    client_roots.insert(path);
                }
            }
        }
        let mut state = self.state.lock().await;
        state.replace_client_roots(client_roots);
    }

    async fn select_root(&self, requested: Option<&str>) -> Result<PathBuf, String> {
        let mut state = self.state.lock().await;
        state.cleanup();
        match requested {
            Some(requested) => state
                .roots
                .iter()
                .find(|root| root.to_string_lossy() == requested)
                .cloned()
                .ok_or_else(|| "root is outside the MCP server allowlist".to_string()),
            None if state.roots.len() == 1 => Ok(state.roots.iter().next().unwrap().clone()),
            None => {
                Err("multiple roots are available; select one returned by inspect_project".into())
            }
        }
    }

    async fn start(&self, root: PathBuf, kind: RunKind) -> Result<RunSnapshot, String> {
        let id = format!("hk-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let cancel = CancellationToken::new();
        let snapshot = {
            let mut state = self.state.lock().await;
            state.cleanup();
            if state
                .runs
                .iter()
                .any(|run| run.root == root && run.active())
            {
                return Err(format!("a run is already active for {}", root.display()));
            }
            let run = RunRecord {
                id: id.clone(),
                root: root.clone(),
                kind,
                status: "starting".into(),
                started_at: chrono::Utc::now().to_rfc3339(),
                finished_at: None,
                completed_at: None,
                exit_code: None,
                output: Vec::new(),
                stdout: Vec::new(),
                stdout_event_buffer: Vec::new(),
                output_truncated: false,
                stdout_truncated: false,
                saw_run_completed: false,
                result: None,
                diff: String::new(),
                diff_truncated: false,
                error: None,
                cancel: cancel.clone(),
            };
            let snapshot = run.snapshot();
            state.runs.push_back(run);
            snapshot
        };
        let server = self.clone();
        tokio::spawn(async move { server.execute(id, root, kind, cancel).await });
        Ok(snapshot)
    }

    async fn execute(&self, id: String, root: PathBuf, kind: RunKind, cancel: CancellationToken) {
        let diff_baseline = snapshot_tree(&root).await;
        let executable = match std::env::current_exe() {
            Ok(path) => path,
            Err(error) => {
                self.finish_error(&id, format!("failed to locate hk executable: {error}"))
                    .await;
                return;
            }
        };
        let mut command = Command::new(executable);
        command
            .arg("--cd")
            .arg(&root)
            .args(["--format", "jsonl"])
            .arg(kind.command())
            .arg("--all")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        if kind.safe() {
            command.arg("--safe");
        }
        if matches!(kind, RunKind::SafeFix) {
            command.arg("--no-stage");
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                self.finish_error(&id, format!("failed to start hk: {error}"))
                    .await;
                return;
            }
        };
        self.set_status(&id, "running").await;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let stdout_task = tokio::spawn(read_output(self.state.clone(), id.clone(), stdout, true));
        let stderr_task = tokio::spawn(read_output(self.state.clone(), id.clone(), stderr, false));
        let (status, cancelled) = tokio::select! {
            status = child.wait() => (status, false),
            _ = cancel.cancelled() => {
                let _ = child.kill().await;
                (child.wait().await, true)
            }
        };
        let _ = stdout_task.await;
        let _ = stderr_task.await;
        let (diff, diff_error) = capture_run_diff(&root, diff_baseline).await;
        let mut state = self.state.lock().await;
        let Some(run) = state.runs.iter_mut().find(|run| run.id == id) else {
            return;
        };
        run.finished_at = Some(chrono::Utc::now().to_rfc3339());
        run.completed_at = Some(Instant::now());
        run.diff = diff.text;
        run.diff_truncated = diff.truncated;
        match status {
            Ok(status) => {
                run.exit_code = status.code();
                let invalid_result = parse_run_result(run);
                run.status = if cancelled {
                    "cancelled"
                } else if invalid_result {
                    "failed"
                } else if status.success() {
                    "succeeded"
                } else {
                    "failed"
                }
                .into();
            }
            Err(error) => {
                run.status = "failed".into();
                run.error = Some(format!("failed to wait for hk: {error}"));
            }
        }
        record_diff_error(run, diff_error);
        state.cleanup();
    }

    async fn set_status(&self, id: &str, status: &str) {
        let mut state = self.state.lock().await;
        if let Some(run) = state.runs.iter_mut().find(|run| run.id == id) {
            run.status = status.into();
        }
    }

    async fn finish_error(&self, id: &str, error: String) {
        let mut state = self.state.lock().await;
        if let Some(run) = state.runs.iter_mut().find(|run| run.id == id) {
            run.status = "failed".into();
            run.error = Some(error);
            run.finished_at = Some(chrono::Utc::now().to_rfc3339());
            run.completed_at = Some(Instant::now());
        }
    }

    async fn snapshot(&self, id: &str) -> Result<RunSnapshot, String> {
        let mut state = self.state.lock().await;
        state.cleanup();
        state
            .runs
            .iter()
            .find(|run| run.id == id)
            .map(RunRecord::snapshot)
            .ok_or_else(|| "run not found or expired".into())
    }

    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    async fn prepare_debug_shutdown(&self) -> usize {
        let mut state = self.state.lock().await;
        state.cleanup();
        let mut active_runs = 0;
        for run in state.runs.iter_mut().filter(|run| run.active()) {
            active_runs += 1;
            run.status = "cancelling".into();
            run.cancel.cancel();
        }
        active_runs
    }
}

fn is_within_startup_roots(startup_roots: &BTreeSet<PathBuf>, path: &Path) -> bool {
    startup_roots
        .iter()
        .any(|startup_root| path.starts_with(startup_root))
}

#[tool_router]
impl HkMcpServer {
    #[tool(
        description = "Inspect the hk project and list roots this server is allowed to access",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn inspect_project(
        &self,
        Parameters(request): Parameters<RootRequest>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.refresh_client_roots(&peer).await;
        let root = self.select_root(request.root.as_deref()).await?;
        let roots = self
            .state
            .lock()
            .await
            .roots
            .iter()
            .map(|root| root.display().to_string())
            .collect::<Vec<_>>();
        let has_config = Config::project_config_exists_from(&root);
        let is_git_repository = is_git_repository(&root).await;
        let value = json!({
            "schema_version": 1,
            "root": root,
            "roots": roots,
            "has_config": has_config,
            "is_git_repository": is_git_repository,
        });
        Ok(tool_success(format!("Inspected {}", root.display()), value))
    }

    #[tool(
        description = "Return hk's execution plan, including command effects, without running steps",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn plan(
        &self,
        Parameters(request): Parameters<RootRequest>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.refresh_client_roots(&peer).await;
        let root = self.select_root(request.root.as_deref()).await?;
        let output = run_hk_capture(&root, &["check", "--all", "--plan", "--json"]).await?;
        let value: Value = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("hk returned an invalid plan: {error}"))?;
        Ok(tool_success(
            format!("Planned checks for {}", root.display()),
            value,
        ))
    }

    #[tool(
        description = "Start an hk check. Legacy commands may have unknown effects.",
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = false,
            open_world_hint = true
        )
    )]
    async fn start_check(
        &self,
        Parameters(request): Parameters<RootRequest>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.start_tool(request, peer, RunKind::Check).await
    }

    #[tool(
        description = "Start an all-or-nothing safe hk check; rejects unknown or destructive commands before execution",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn start_safe_check(
        &self,
        Parameters(request): Parameters<RootRequest>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.start_tool(request, peer, RunKind::SafeCheck).await
    }

    #[tool(
        description = "Start a confirmed safe hk fix; rejects unknown or destructive commands and never stages changes",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn start_safe_fix(
        &self,
        Parameters(request): Parameters<RootRequest>,
        peer: Peer<RoleServer>,
    ) -> Result<CallToolResult, String> {
        self.start_tool(request, peer, RunKind::SafeFix).await
    }

    #[tool(
        description = "Get the current authoritative state and structured result for a run",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_run(
        &self,
        Parameters(request): Parameters<RunRequest>,
    ) -> Result<CallToolResult, String> {
        let snapshot = self.snapshot(&request.run_id).await?;
        let value = serde_json::to_value(&snapshot).map_err(|error| error.to_string())?;
        Ok(tool_success(
            format!("Run {} is {}", snapshot.id, snapshot.status),
            value,
        ))
    }

    #[tool(
        description = "Read a byte-paged chunk of run logs (maximum 64 KiB per call)",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_output(
        &self,
        Parameters(request): Parameters<OutputRequest>,
    ) -> Result<CallToolResult, String> {
        let mut state = self.state.lock().await;
        state.cleanup();
        let run = state
            .runs
            .iter()
            .find(|run| run.id == request.run_id)
            .ok_or_else(|| "run not found or expired".to_string())?;
        let requested_offset = request.offset.min(run.output.len());
        let limit = request
            .limit
            .unwrap_or(OUTPUT_PAGE_MAX)
            .clamp(1, OUTPUT_PAGE_MAX);
        let (offset, end) = utf8_page_bounds(&run.output, requested_offset, limit);
        let page = OutputPage {
            schema_version: 1,
            run_id: run.id.clone(),
            offset,
            next_offset: end,
            total_bytes: run.output.len(),
            eof: end == run.output.len(),
            truncated: run.output_truncated,
            text: String::from_utf8_lossy(&run.output[offset..end]).into_owned(),
        };
        let value = serde_json::to_value(&page).map_err(|error| error.to_string())?;
        Ok(tool_success(
            format!("Run output bytes {offset}..{end} of {}", run.output.len()),
            value,
        ))
    }

    #[tool(
        description = "Read a byte-paged chunk of the Git patch captured when a run completed (maximum 64 KiB per call)",
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_diff(
        &self,
        Parameters(request): Parameters<OutputRequest>,
    ) -> Result<CallToolResult, String> {
        let mut state = self.state.lock().await;
        state.cleanup();
        let run = state
            .runs
            .iter()
            .find(|run| run.id == request.run_id)
            .ok_or_else(|| "run not found or expired".to_string())?;
        let bytes = run.diff.as_bytes();
        let mut offset = request.offset.min(bytes.len());
        while offset < bytes.len() && !run.diff.is_char_boundary(offset) {
            offset += 1;
        }
        let limit = request
            .limit
            .unwrap_or(OUTPUT_PAGE_MAX)
            .clamp(4, OUTPUT_PAGE_MAX);
        let mut end = offset.saturating_add(limit).min(bytes.len());
        while end > offset && !run.diff.is_char_boundary(end) {
            end -= 1;
        }
        let page = OutputPage {
            schema_version: 1,
            run_id: run.id.clone(),
            offset,
            next_offset: end,
            total_bytes: bytes.len(),
            eof: end == bytes.len(),
            truncated: run.diff_truncated,
            text: run.diff[offset..end].to_string(),
        };
        let value = serde_json::to_value(&page).map_err(|error| error.to_string())?;
        Ok(tool_success(
            format!("Run patch bytes {offset}..{end} of {}", bytes.len()),
            value,
        ))
    }

    #[tool(
        description = "Cancel an active hk run",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn cancel_run(
        &self,
        Parameters(request): Parameters<RunRequest>,
    ) -> Result<CallToolResult, String> {
        let mut state = self.state.lock().await;
        state.cleanup();
        let run = state
            .runs
            .iter_mut()
            .find(|run| run.id == request.run_id)
            .ok_or_else(|| "run not found or expired".to_string())?;
        if run.active() {
            run.status = "cancelling".into();
            run.cancel.cancel();
        }
        let snapshot = run.snapshot();
        let value = serde_json::to_value(&snapshot).map_err(|error| error.to_string())?;
        Ok(tool_success(
            format!("Run {} is {}", snapshot.id, snapshot.status),
            value,
        ))
    }

    #[tool(
        description = "Render a run for an MCP Apps host, with structured fallback for other clients",
        meta = dashboard_tool_meta(),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn render_run(
        &self,
        Parameters(request): Parameters<RunRequest>,
    ) -> Result<CallToolResult, String> {
        let snapshot = self.snapshot(&request.run_id).await?;
        let value = json!({
            "schema_version": 1,
            "view": "hk.run",
            "run": snapshot,
            "ui_available": true,
        });
        Ok(tool_success(
            "Run view is available as structured content".into(),
            value,
        ))
    }
}

fn utf8_page_bounds(bytes: &[u8], requested_offset: usize, limit: usize) -> (usize, usize) {
    let mut offset = requested_offset.min(bytes.len());
    while offset < bytes.len() && bytes[offset] & 0b1100_0000 == 0b1000_0000 {
        offset += 1;
    }
    let mut end = offset.saturating_add(limit).min(bytes.len());
    while end > offset && end < bytes.len() && bytes[end] & 0b1100_0000 == 0b1000_0000 {
        end -= 1;
    }
    if end == offset && offset < bytes.len() {
        end = offset + 1;
        while end < bytes.len() && bytes[end] & 0b1100_0000 == 0b1000_0000 {
            end += 1;
        }
    }
    (offset, end)
}

impl HkMcpServer {
    fn combined_tool_router() -> ToolRouter<Self> {
        let router = Self::tool_router();
        #[cfg(debug_assertions)]
        {
            router + Self::debug_tool_router()
        }
        #[cfg(not(debug_assertions))]
        {
            router
        }
    }

    async fn start_tool(
        &self,
        request: RootRequest,
        peer: Peer<RoleServer>,
        kind: RunKind,
    ) -> Result<CallToolResult, String> {
        self.refresh_client_roots(&peer).await;
        let root = self.select_root(request.root.as_deref()).await?;
        let snapshot = self.start(root, kind).await?;
        let value = serde_json::to_value(&snapshot).map_err(|error| error.to_string())?;
        Ok(tool_success(format!("Started run {}", snapshot.id), value))
    }
}

#[cfg(debug_assertions)]
#[tool_router(router = debug_tool_router)]
impl HkMcpServer {
    #[tool(
        description = "Shut down this debug MCP server after replying; active runs are terminated and the host must reconnect",
        annotations(
            read_only_hint = false,
            destructive_hint = false,
            idempotent_hint = false,
            open_world_hint = false
        )
    )]
    async fn debug_shutdown(&self) -> Result<CallToolResult, String> {
        let active_runs = self.prepare_debug_shutdown().await;
        let state = self.state.clone();
        tokio::spawn(async move {
            loop {
                let runs_stopped = {
                    let state = state.lock().await;
                    !state.runs.iter().any(RunRecord::active)
                };
                if runs_stopped {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
            std::process::exit(0);
        });
        Ok(tool_success(
            format!(
                "Debug MCP server is shutting down after {active_runs} active run(s) stop; reconnect it in the host"
            ),
            json!({
                "schema_version": 1,
                "status": "shutting_down",
                "active_runs": active_runs,
            }),
        ))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for HkMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
        )
            .with_server_info(Implementation::new("hk", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Inspect and run hk within authorized project roots. Prefer plan and safe tools; review diffs after fixes.",
            )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let resource = Resource::new(DASHBOARD_URI, "hk-run-dashboard")
            .with_title("hk run dashboard")
            .with_description(
                "Interactive status, diagnostics, logs, and patch review for an hk run",
            )
            .with_mime_type(MCP_APP_MIME)
            .with_size(DASHBOARD_HTML.len() as u64)
            .with_meta(
                serde_json::from_value(json!({
                    "ui": {
                        "csp": {"connectDomains": [], "resourceDomains": []},
                        "prefersBorder": true
                    }
                }))
                .expect("dashboard resource metadata is an object"),
            );
        Ok(ListResourcesResult::with_all_items(vec![resource]))
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        if request.uri != DASHBOARD_URI {
            return Err(ErrorData::resource_not_found("resource not found", None));
        }
        Ok(ReadResourceResult::new(vec![
            ResourceContents::text(DASHBOARD_HTML, DASHBOARD_URI).with_mime_type(MCP_APP_MIME),
        ])
        .into())
    }
}

fn tool_success(summary: String, value: Value) -> CallToolResult {
    let mut result = CallToolResult::success(vec![ContentBlock::text(summary)]);
    result.structured_content = Some(value);
    result
}

fn parse_run_result(run: &mut RunRecord) -> bool {
    if run.saw_run_completed && run.result.is_some() {
        return false;
    }
    if run.stdout_truncated {
        run.error = Some(format!(
            "structured result exceeded the {} byte capture limit",
            MAX_RUN_OUTPUT_BYTES
        ));
        return true;
    }
    if run.error.is_none() {
        run.error =
            Some("failed to parse hk structured result: missing run_completed event".into());
    }
    true
}

fn apply_jsonl_event(run: &mut RunRecord, line: &[u8]) {
    let event: Value = match serde_json::from_slice(line) {
        Ok(event) => event,
        Err(error) => {
            run.error = Some(format!("failed to parse hk structured result: {error}"));
            return;
        }
    };
    let Some(kind) = event.get("event").and_then(Value::as_str) else {
        run.error = Some("failed to parse hk structured result: event name is missing".into());
        return;
    };
    let Some(data) = event.get("data").cloned() else {
        run.error = Some("failed to parse hk structured result: event data is missing".into());
        return;
    };
    match kind {
        "run_started" => {
            run.result = Some(json!({
                "schema_version": 1,
                "kind": "run_result",
                "hook": data.get("hook").and_then(Value::as_str).unwrap_or(run.kind.command()),
                "status": "running",
                "started_at": data.get("started_at").and_then(Value::as_str).unwrap_or(&run.started_at),
                "duration_ms": 0,
                "steps": [],
            }));
        }
        "run_planned" => {
            let Some(steps) = data.get("steps").and_then(Value::as_array) else {
                run.error =
                    Some("failed to parse hk structured result: planned steps are invalid".into());
                return;
            };
            if let Some(result) = run.result.as_mut() {
                result["steps"] = Value::Array(steps.clone());
            }
        }
        "step_started" | "step_completed" => {
            let Some(name) = data.get("name").and_then(Value::as_str) else {
                run.error =
                    Some("failed to parse hk structured result: step name is missing".into());
                return;
            };
            let fallback = || {
                json!({
                    "schema_version": 1,
                    "kind": "run_result",
                    "hook": run.kind.command(),
                    "status": "running",
                    "started_at": run.started_at,
                    "duration_ms": 0,
                    "steps": [],
                })
            };
            let result = run.result.get_or_insert_with(fallback);
            let Some(steps) = result.get_mut("steps").and_then(Value::as_array_mut) else {
                run.error = Some("failed to parse hk structured result: steps are invalid".into());
                return;
            };
            if let Some(existing) = steps
                .iter_mut()
                .find(|step| step.get("name").and_then(Value::as_str) == Some(name))
            {
                *existing = data;
            } else {
                steps.push(data);
            }
        }
        "run_completed" => {
            run.result = Some(data);
            run.saw_run_completed = true;
        }
        _ => {}
    }
}

fn consume_jsonl_events(run: &mut RunRecord, bytes: &[u8]) {
    run.stdout_event_buffer.extend_from_slice(bytes);
    while let Some(newline) = run
        .stdout_event_buffer
        .iter()
        .position(|byte| *byte == b'\n')
    {
        let mut line = run
            .stdout_event_buffer
            .drain(..=newline)
            .collect::<Vec<_>>();
        line.pop();
        if !line.iter().all(u8::is_ascii_whitespace) {
            apply_jsonl_event(run, &line);
        }
    }
}

async fn read_output<R>(state: Arc<Mutex<McpState>>, id: String, mut reader: R, stdout: bool)
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 8192];
    loop {
        let count = match reader.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(count) => count,
        };
        let mut state = state.lock().await;
        let Some(run) = state.runs.iter_mut().find(|run| run.id == id) else {
            break;
        };
        if stdout {
            let previous_len = run.stdout.len();
            if append_capped(&mut run.stdout, &buffer[..count]) {
                run.stdout_truncated = true;
            }
            let appended = run.stdout[previous_len..].to_vec();
            consume_jsonl_events(run, &appended);
        } else if append_capped(&mut run.output, &buffer[..count]) {
            run.output_truncated = true;
        }
    }
}

fn append_capped(target: &mut Vec<u8>, bytes: &[u8]) -> bool {
    let remaining = MAX_RUN_OUTPUT_BYTES.saturating_sub(target.len());
    target.extend_from_slice(&bytes[..bytes.len().min(remaining)]);
    bytes.len() > remaining
}

async fn run_hk_capture(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let output = Command::new(executable)
        .arg("--cd")
        .arg(root)
        .args(args)
        .stdin(std::process::Stdio::null())
        .output()
        .await
        .map_err(|error| format!("failed to start hk: {error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "hk failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

async fn is_git_repository(root: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--git-dir"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await
        .is_ok_and(|status| status.success())
}

#[derive(Debug, Default)]
struct CapturedDiff {
    text: String,
    truncated: bool,
}

async fn run_git(root: &Path, args: &[&str], index: Option<&Path>) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    if let Some(index) = index {
        command.env("GIT_INDEX_FILE", index);
    }
    let output = command
        .stdin(std::process::Stdio::null())
        .output()
        .await
        .map_err(|error| format!("failed to run git {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

/// Records the working tree (tracked and untracked, excluding ignored files) as a git
/// tree object using a private temporary index, so the user's real index is never touched.
async fn snapshot_tree(root: &Path) -> Result<String, String> {
    let index_path = run_git(root, &["rev-parse", "--git-path", "index"], None).await?;
    let index_path = PathBuf::from(String::from_utf8_lossy(&index_path).trim());
    let index_path = if index_path.is_absolute() {
        index_path
    } else {
        root.join(index_path)
    };
    // The temp index lives in its own directory (removed on drop) because git rejects an
    // empty pre-created index file; it starts out absent when there is no real index.
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let temp_index = temp_dir.path().join("index");
    if index_path.exists() {
        std::fs::copy(&index_path, &temp_index)
            .map_err(|error| format!("failed to copy git index: {error}"))?;
    } else {
        run_git(root, &["read-tree", "--empty"], Some(&temp_index)).await?;
    }
    drop_unmerged_entries(root, &temp_index).await?;
    // Store the worktree bytes as they are: no CRLF conversion, and no clean filters
    // (those come from attributes, which are read from the empty tree instead). Only
    // paths under `root` are re-read, so changes elsewhere in the repository cannot
    // show up in the diff. Git older than 2.40 lacks `--attr-source`.
    let raw = ["-c", "core.autocrlf=false", "-c", "core.safecrlf=false"];
    let mut with_attr_source = raw.to_vec();
    let attr_source = format!("--attr-source={EMPTY_TREE}");
    with_attr_source.push(&attr_source);
    with_attr_source.extend(["add", "-A", "--", "."]);
    if run_git(root, &with_attr_source, Some(&temp_index))
        .await
        .is_err()
    {
        let mut without = raw.to_vec();
        without.extend(["add", "-A", "--", "."]);
        run_git(root, &without, Some(&temp_index)).await?;
    }
    let tree = run_git(root, &["write-tree"], Some(&temp_index)).await?;
    Ok(String::from_utf8_lossy(&tree).trim().to_string())
}

/// `write-tree` refuses an index with unmerged entries, such as a merge conflict in a
/// file outside `root` (which the pathspec-limited `add` never refreshes). Both snapshots
/// of a run drop the same paths, so they cannot show up in the diff.
async fn drop_unmerged_entries(root: &Path, index: &Path) -> Result<(), String> {
    let top = run_git(root, &["rev-parse", "--show-toplevel"], None).await?;
    let top = PathBuf::from(String::from_utf8_lossy(&top).trim());
    let unmerged = run_git(&top, &["ls-files", "-u", "-z"], Some(index)).await?;
    let mut paths = Vec::new();
    for entry in unmerged.split(|byte| *byte == 0) {
        // `<mode> <object> <stage>\t<path>`
        if let Some(tab) = entry.iter().position(|byte| *byte == b'\t') {
            let path = String::from_utf8_lossy(&entry[tab + 1..]).into_owned();
            if paths.last() != Some(&path) {
                paths.push(path);
            }
        }
    }
    for chunk in paths.chunks(256) {
        let mut args = vec!["update-index", "--force-remove", "--"];
        args.extend(chunk.iter().map(String::as_str));
        run_git(&top, &args, Some(index)).await?;
    }
    Ok(())
}

/// Git's well-known empty tree, which always exists.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

async fn diff_tree_bytes(
    root: &Path,
    before: &str,
    after: &str,
    attributes: Option<&Path>,
    ignore_repo_attributes: bool,
) -> Result<(Vec<u8>, bool), String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root);
    if let Some(attributes) = attributes {
        let mut setting = std::ffi::OsString::from("core.attributesFile=");
        setting.push(attributes);
        command.arg("-c").arg(setting);
    }
    if ignore_repo_attributes {
        command.arg(format!("--attr-source={EMPTY_TREE}"));
    }
    let mut child = command
        .args([
            "diff-tree",
            "-p",
            "--binary",
            "--no-ext-diff",
            "--no-textconv",
            "--relative",
            before,
            after,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("failed to run git diff-tree: {error}"))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "failed to capture git diff output".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "failed to capture git diff errors".to_string())?;
    let stderr_task = tokio::spawn(async move {
        let mut text = Vec::new();
        let _ = stderr.read_to_end(&mut text).await;
        text
    });
    let mut bytes = Vec::new();
    let mut truncated = false;
    let mut buffer = [0_u8; 8192];
    loop {
        let count = stdout
            .read(&mut buffer)
            .await
            .map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        let remaining = MAX_RUN_DIFF_BYTES.saturating_sub(bytes.len());
        bytes.extend_from_slice(&buffer[..count.min(remaining)]);
        truncated |= count > remaining;
    }
    let status = child.wait().await.map_err(|error| error.to_string())?;
    let stderr = stderr_task.await.unwrap_or_default();
    if !status.success() {
        return Err(format!(
            "git diff-tree failed: {}",
            String::from_utf8_lossy(&stderr).trim()
        ));
    }
    Ok((bytes, truncated))
}

/// Diffs two trees from `snapshot_tree`. Output that is not valid UTF-8 (text files in
/// other encodings) is regenerated with every file encoded as a base64 binary patch so
/// no bytes are altered and the result still applies with `git apply`.
async fn git_diff(root: &Path, before: &str, after: &str) -> Result<CapturedDiff, String> {
    let (bytes, truncated) = diff_tree_bytes(root, before, after, None, false).await?;
    if let Some(text) = utf8_patch(&bytes, truncated) {
        return Ok(CapturedDiff { text, truncated });
    }
    let temp_dir = tempfile::tempdir().map_err(|error| error.to_string())?;
    let attributes = temp_dir.path().join("attributes");
    std::fs::write(&attributes, "* -diff\n").map_err(|error| error.to_string())?;
    // Repository attributes override `core.attributesFile`, so ignore them too.
    let (bytes, truncated) =
        match diff_tree_bytes(root, before, after, Some(&attributes), true).await {
            Ok(result) => result,
            Err(_) => diff_tree_bytes(root, before, after, Some(&attributes), false).await?,
        };
    match utf8_patch(&bytes, truncated) {
        Some(text) => Ok(CapturedDiff { text, truncated }),
        None => Err("the patch is not valid UTF-8 even as a binary patch".into()),
    }
}

/// The patch as text when it is valid UTF-8; a capture cut off mid-character keeps the
/// complete characters before the cut.
fn utf8_patch(bytes: &[u8], truncated: bool) -> Option<String> {
    match std::str::from_utf8(bytes) {
        Ok(text) => Some(text.to_string()),
        Err(error) if truncated && error.error_len().is_none() => {
            Some(String::from_utf8_lossy(&bytes[..error.valid_up_to()]).into_owned())
        }
        Err(_) => None,
    }
}

/// Diffs the working tree against the snapshot taken before a run. A failure is
/// returned beside an empty diff so the run can report it.
async fn capture_run_diff(
    root: &Path,
    baseline: Result<String, String>,
) -> (CapturedDiff, Option<String>) {
    let result = async {
        let before = baseline?;
        let after = snapshot_tree(root).await?;
        git_diff(root, &before, &after).await
    }
    .await;
    match result {
        Ok(diff) => (diff, None),
        Err(error) => (CapturedDiff::default(), Some(error)),
    }
}

fn record_diff_error(run: &mut RunRecord, error: Option<String>) {
    if let Some(error) = error {
        let diff_error = format!("failed to capture diff: {error}");
        run.error = Some(match run.error.take() {
            Some(existing) => format!("{existing}; {diff_error}"),
            None => diff_error,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    fn test_run(id: &str, status: &str, output: Vec<u8>) -> RunRecord {
        RunRecord {
            id: id.into(),
            root: PathBuf::from("/project"),
            kind: RunKind::Check,
            status: status.into(),
            started_at: String::new(),
            finished_at: (!matches!(status, "starting" | "running" | "cancelling"))
                .then(String::new),
            completed_at: (!matches!(status, "starting" | "running" | "cancelling"))
                .then(Instant::now),
            exit_code: None,
            output,
            stdout: Vec::new(),
            stdout_event_buffer: Vec::new(),
            output_truncated: false,
            stdout_truncated: false,
            saw_run_completed: false,
            result: None,
            diff: String::new(),
            diff_truncated: false,
            error: None,
            cancel: CancellationToken::new(),
        }
    }

    #[test]
    fn rejects_files_as_roots() {
        let file = tempfile::NamedTempFile::new().unwrap();
        assert!(canonical_directory(file.path()).is_err());
    }

    #[tokio::test]
    async fn roots_cannot_be_introduced_by_tool_arguments() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        assert!(server.select_root(Some("/not/authorized")).await.is_err());
    }

    #[test]
    fn client_roots_must_remain_within_the_startup_boundary() {
        let directory = tempfile::tempdir().unwrap();
        let startup = directory.path().canonicalize().unwrap();
        let child = startup.join("child");
        std::fs::create_dir(&child).unwrap();
        let sibling = tempfile::tempdir().unwrap().path().canonicalize().unwrap();
        let state = McpState {
            startup_roots: BTreeSet::from([startup.clone()]),
            roots: BTreeSet::from([startup]),
            runs: VecDeque::new(),
        };

        assert!(is_within_startup_roots(&state.startup_roots, &child));
        assert!(!is_within_startup_roots(&state.startup_roots, &sibling));
    }

    #[test]
    fn replacing_client_roots_removes_stale_entries() {
        let directory = tempfile::tempdir().unwrap();
        let startup = directory.path().canonicalize().unwrap();
        let stale = startup.join("stale");
        let current = startup.join("current");
        let mut state = McpState {
            startup_roots: BTreeSet::from([startup.clone()]),
            roots: BTreeSet::from([startup.clone(), stale]),
            runs: VecDeque::new(),
        };

        state.replace_client_roots(BTreeSet::from([current.clone()]));

        assert_eq!(state.roots, BTreeSet::from([startup.clone(), current]));

        state.replace_client_roots(BTreeSet::new());
        assert_eq!(state.roots, BTreeSet::from([startup]));
    }

    #[tokio::test]
    async fn diff_is_scoped_to_changes_after_the_run_baseline() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .status()
            .unwrap();
        std::fs::write(root.join("before.txt"), "clean\n").unwrap();
        std::fs::write(root.join("during.txt"), "clean\n").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .status()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-c",
                "user.name=hk",
                "-c",
                "user.email=hk@example.com",
                "commit",
                "-qm",
                "init",
            ])
            .current_dir(root)
            .status()
            .unwrap();
        std::fs::write(root.join("before.txt"), "pre-existing\n").unwrap();
        let baseline = snapshot_tree(root).await.unwrap();

        std::fs::write(root.join("during.txt"), "changed by run\n").unwrap();
        let after = snapshot_tree(root).await.unwrap();
        let diff = git_diff(root, &baseline, &after).await.unwrap();

        assert!(diff.text.contains("during.txt"));
        assert!(!diff.text.contains("before.txt"));
    }

    fn git_in(root: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[tokio::test]
    async fn diff_includes_untracked_files_and_leaves_the_real_index_alone() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        git_in(root, &["init", "-q"]);
        std::fs::write(root.join("tracked.txt"), "x\n").unwrap();
        git_in(root, &["add", "."]);
        let index_before = std::fs::read(root.join(".git/index")).unwrap();
        let baseline = snapshot_tree(root).await.unwrap();

        std::fs::write(root.join("new.txt"), "created by run\n").unwrap();
        let after = snapshot_tree(root).await.unwrap();
        let diff = git_diff(root, &baseline, &after).await.unwrap();

        assert!(diff.text.contains("+++ b/new.txt"));
        assert!(diff.text.contains("+created by run"));
        assert_eq!(
            std::fs::read(root.join(".git/index")).unwrap(),
            index_before
        );
    }

    #[tokio::test]
    async fn diff_works_without_a_git_index() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        git_in(root, &["init", "-q"]);
        assert!(!root.join(".git/index").exists());
        let baseline = snapshot_tree(root).await.unwrap();

        std::fs::write(root.join("new.txt"), "hello\n").unwrap();
        let after = snapshot_tree(root).await.unwrap();
        let diff = git_diff(root, &baseline, &after).await.unwrap();

        assert!(diff.text.contains("+++ b/new.txt"));
        assert!(!root.join(".git/index").exists());
    }

    #[tokio::test]
    async fn diff_preserves_binary_and_non_utf8_content() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        git_in(root, &["init", "-q"]);
        let baseline = snapshot_tree(root).await.unwrap();

        std::fs::write(root.join("latin1.txt"), b"caf\xe9\n").unwrap();
        std::fs::write(root.join("blob.bin"), [0_u8, 159, 146, 150, 255]).unwrap();
        let after = snapshot_tree(root).await.unwrap();
        let diff = git_diff(root, &baseline, &after).await.unwrap();
        assert!(!diff.text.contains('\u{fffd}'));
        assert!(diff.text.contains("GIT binary patch"));

        // The captured patch must reproduce the files byte for byte.
        let patch = root.join("run.patch");
        std::fs::write(&patch, &diff.text).unwrap();
        std::fs::remove_file(root.join("latin1.txt")).unwrap();
        std::fs::remove_file(root.join("blob.bin")).unwrap();
        git_in(root, &["-c", "core.autocrlf=false", "apply", "run.patch"]);
        assert_eq!(
            std::fs::read(root.join("latin1.txt")).unwrap(),
            b"caf\xe9\n"
        );
        assert_eq!(
            std::fs::read(root.join("blob.bin")).unwrap(),
            [0_u8, 159, 146, 150, 255]
        );
    }

    #[tokio::test]
    async fn snapshot_failure_is_an_error_not_an_empty_diff() {
        let directory = tempfile::tempdir().unwrap();
        assert!(snapshot_tree(directory.path()).await.is_err());
        let error = git_diff(directory.path(), "deadbeef", "cafebabe")
            .await
            .unwrap_err();
        assert!(error.contains("diff-tree"));
    }

    #[tokio::test]
    async fn diff_from_a_subdirectory_root_stays_inside_it() {
        let directory = tempfile::tempdir().unwrap();
        let repo = directory.path();
        git_in(repo, &["init", "-q"]);
        let root = repo.join("app");
        std::fs::create_dir(&root).unwrap();
        let baseline = snapshot_tree(&root).await.unwrap();

        std::fs::write(root.join("inside.txt"), "in\n").unwrap();
        std::fs::write(repo.join("outside.txt"), "out\n").unwrap();
        let after = snapshot_tree(&root).await.unwrap();
        let diff = git_diff(&root, &baseline, &after).await.unwrap();

        assert!(diff.text.contains("+++ b/inside.txt"), "{}", diff.text);
        assert!(!diff.text.contains("outside.txt"), "{}", diff.text);
    }

    #[tokio::test]
    async fn snapshots_survive_a_merge_conflict_outside_the_root() {
        let directory = tempfile::tempdir().unwrap();
        let repo = directory.path();
        git_in(repo, &["init", "-q"]);
        git_in(repo, &["config", "user.email", "t@t"]);
        git_in(repo, &["config", "user.name", "t"]);
        std::fs::write(repo.join("conflict.txt"), "base\n").unwrap();
        git_in(repo, &["add", "."]);
        git_in(repo, &["commit", "-qm", "base"]);
        git_in(repo, &["checkout", "-qb", "other"]);
        std::fs::write(repo.join("conflict.txt"), "other\n").unwrap();
        git_in(repo, &["commit", "-qam", "other"]);
        git_in(repo, &["checkout", "-q", "-"]);
        std::fs::write(repo.join("conflict.txt"), "mine\n").unwrap();
        git_in(repo, &["commit", "-qam", "mine"]);
        // Leaves unmerged stages in the real index; the merge itself fails.
        let _ = std::process::Command::new("git")
            .args(["merge", "other"])
            .current_dir(repo)
            .output()
            .unwrap();
        let root = repo.join("app");
        std::fs::create_dir(&root).unwrap();

        let baseline = snapshot_tree(&root).await.unwrap();
        std::fs::write(root.join("new.txt"), "x\n").unwrap();
        let after = snapshot_tree(&root).await.unwrap();
        let diff = git_diff(&root, &baseline, &after).await.unwrap();
        assert!(diff.text.contains("+++ b/new.txt"), "{}", diff.text);
        assert!(!diff.text.contains("conflict.txt"), "{}", diff.text);
    }

    #[tokio::test]
    async fn diff_keeps_worktree_bytes_despite_line_ending_settings() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        git_in(root, &["init", "-q"]);
        git_in(root, &["config", "core.autocrlf", "true"]);
        std::fs::write(root.join(".gitattributes"), "*.txt text eol=lf\n").unwrap();
        let baseline = snapshot_tree(root).await.unwrap();

        std::fs::write(root.join("crlf.txt"), b"a\r\nb\r\n").unwrap();
        let after = snapshot_tree(root).await.unwrap();
        let diff = git_diff(root, &baseline, &after).await.unwrap();

        assert!(diff.text.contains("+a\r\n"), "{:?}", diff.text);
    }

    #[tokio::test]
    async fn capture_failures_are_attached_to_the_run() {
        let directory = tempfile::tempdir().unwrap();
        let (diff, error) = capture_run_diff(directory.path(), Err("no baseline".into())).await;
        assert!(diff.text.is_empty());
        let mut run = test_run("failed-diff", "succeeded", Vec::new());
        record_diff_error(&mut run, error);
        assert_eq!(
            run.error.as_deref(),
            Some("failed to capture diff: no baseline")
        );
        run.error = Some("hk failed".into());
        record_diff_error(&mut run, Some("again".into()));
        assert_eq!(
            run.error.as_deref(),
            Some("hk failed; failed to capture diff: again")
        );
        record_diff_error(&mut run, None);
        assert!(run.error.unwrap().ends_with("again"));
    }

    #[test]
    fn utf8_patch_rejects_invalid_bytes_but_trims_a_truncated_character() {
        assert_eq!(utf8_patch("é".as_bytes(), false).as_deref(), Some("é"));
        assert_eq!(utf8_patch(&"é".as_bytes()[..1], true).as_deref(), Some(""));
        assert!(utf8_patch(b"caf\xe9\n", false).is_none());
        assert!(utf8_patch(b"caf\xe9\n", true).is_none());
    }

    #[test]
    fn cleanup_retains_only_32_completed_runs() {
        let root = PathBuf::from("/project");
        let mut state = McpState::default();
        for index in 0..40 {
            let mut run = test_run(&index.to_string(), "succeeded", Vec::new());
            run.root = root.clone();
            run.exit_code = Some(0);
            state.runs.push_back(run);
        }
        state.cleanup();
        assert_eq!(state.runs.len(), COMPLETED_RUN_LIMIT);
        assert_eq!(state.runs.front().unwrap().id, "8");
    }

    #[test]
    fn output_page_limit_is_64_kib() {
        assert_eq!(OUTPUT_PAGE_MAX, 65_536);
    }

    #[test]
    fn project_config_detection_excludes_removed_formats() {
        let directory = tempfile::tempdir().unwrap();
        let child = directory.path().join("nested");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(directory.path().join("hk.yaml"), "hooks: {}\n").unwrap();

        assert!(!Config::project_config_exists_from(&child));

        std::fs::remove_file(directory.path().join("hk.yaml")).unwrap();
        std::fs::create_dir(directory.path().join("hk.yaml")).unwrap();
        assert!(!Config::project_config_exists_from(&child));
    }

    #[tokio::test]
    async fn git_repository_detection_works_from_a_subdirectory() {
        let directory = tempfile::tempdir().unwrap();
        let child = directory.path().join("nested");
        std::fs::create_dir(&child).unwrap();
        assert!(
            std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(directory.path())
                .status()
                .unwrap()
                .success()
        );

        assert!(is_git_repository(&child).await);
    }

    #[tokio::test]
    async fn structured_output_is_capped_without_polluting_run_logs() {
        let mut state = McpState::default();
        state
            .runs
            .push_back(test_run("large", "running", Vec::new()));
        let state = Arc::new(Mutex::new(state));
        let input = vec![b'x'; MAX_RUN_OUTPUT_BYTES + 1024];

        read_output(state.clone(), "large".into(), input.as_slice(), true).await;

        let state = state.lock().await;
        let run = &state.runs[0];
        assert!(run.output.is_empty());
        assert_eq!(run.stdout.len(), MAX_RUN_OUTPUT_BYTES);
        assert!(!run.output_truncated);
        assert!(run.stdout_truncated);
    }

    #[test]
    fn malformed_structured_output_is_a_run_error() {
        let mut run = test_run("malformed", "running", Vec::new());
        consume_jsonl_events(&mut run, b"not json\n");

        assert!(parse_run_result(&mut run));
        assert!(run.result.is_none());
        assert!(
            run.error
                .as_deref()
                .unwrap()
                .starts_with("failed to parse hk structured result:")
        );
    }

    #[test]
    fn valid_structured_output_is_retained() {
        let mut run = test_run("valid", "running", Vec::new());
        consume_jsonl_events(
            &mut run,
            br#"{"schema_version":1,"event":"run_completed","sequence":1,"data":{"schema_version":1,"kind":"run_result","status":"passed","steps":[]}}
"#,
        );

        assert!(!parse_run_result(&mut run));
        assert_eq!(run.result.as_ref().unwrap()["status"], "passed");
        assert!(run.error.is_none());
    }

    #[test]
    fn parsed_completion_survives_trailing_structured_output_truncation() {
        let mut run = test_run("valid-truncated", "running", Vec::new());
        consume_jsonl_events(
            &mut run,
            br#"{"schema_version":1,"event":"run_completed","sequence":1,"data":{"schema_version":1,"kind":"run_result","status":"passed","steps":[]}}
"#,
        );
        run.stdout_truncated = true;

        assert!(!parse_run_result(&mut run));
        assert_eq!(run.result.as_ref().unwrap()["status"], "passed");
        assert!(run.error.is_none());
    }

    #[test]
    fn step_started_event_is_visible_before_run_completion() {
        let mut run = test_run("live", "running", Vec::new());
        consume_jsonl_events(
            &mut run,
            br#"{"schema_version":1,"event":"run_started","sequence":0,"data":{"hook":"check","started_at":"now"}}
{"schema_version":1,"event":"step_started","sequence":1,"data":{"name":"cargo-check","status":"running","duration_ms":0,"effects":[],"diagnostics":[]}}
"#,
        );

        let result = run.result.as_ref().unwrap();
        assert_eq!(result["status"], "running");
        assert_eq!(result["steps"][0]["name"], "cargo-check");
        assert_eq!(result["steps"][0]["status"], "running");
        assert!(!run.saw_run_completed);
    }

    #[test]
    fn cleanup_expires_old_completed_runs_but_not_active_runs() {
        let mut state = McpState::default();
        let mut expired = test_run("expired", "succeeded", Vec::new());
        let completed_at = Instant::now();
        expired.completed_at = Some(completed_at);
        state.runs.push_back(expired);
        state
            .runs
            .push_back(test_run("active", "running", Vec::new()));
        state.cleanup_at(completed_at + COMPLETED_RUN_TTL + Duration::from_secs(1));
        assert_eq!(state.runs.len(), 1);
        assert_eq!(state.runs[0].id, "active");
    }

    #[tokio::test]
    async fn output_is_byte_paged_and_capped() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        server.state.lock().await.runs.push_back(test_run(
            "paged",
            "succeeded",
            vec![b'x'; OUTPUT_PAGE_MAX + 17],
        ));
        let result = server
            .get_output(Parameters(OutputRequest {
                run_id: "paged".into(),
                offset: 0,
                limit: Some(OUTPUT_PAGE_MAX * 2),
            }))
            .await
            .unwrap();
        let value = result.structured_content.unwrap();
        assert_eq!(value["next_offset"], OUTPUT_PAGE_MAX);
        assert_eq!(value["eof"], false);
    }

    #[tokio::test]
    async fn output_pagination_advances_when_limit_is_zero() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        server
            .state
            .lock()
            .await
            .runs
            .push_back(test_run("zero", "succeeded", b"abc".to_vec()));

        let result = server
            .get_output(Parameters(OutputRequest {
                run_id: "zero".into(),
                offset: 0,
                limit: Some(0),
            }))
            .await
            .unwrap();
        let value = result.structured_content.unwrap();
        assert_eq!(value["offset"], 0);
        assert_eq!(value["next_offset"], 1);
        assert_eq!(value["text"], "a");
    }

    #[tokio::test]
    async fn output_pagination_does_not_split_a_multibyte_character() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        server.state.lock().await.runs.push_back(test_run(
            "unicode-output",
            "succeeded",
            "éx".as_bytes().to_vec(),
        ));

        let first = server
            .get_output(Parameters(OutputRequest {
                run_id: "unicode-output".into(),
                offset: 0,
                limit: Some(1),
            }))
            .await
            .unwrap()
            .structured_content
            .unwrap();
        assert_eq!(first["offset"], 0);
        assert_eq!(first["next_offset"], 2);
        assert_eq!(first["text"], "é");

        let second = server
            .get_output(Parameters(OutputRequest {
                run_id: "unicode-output".into(),
                offset: 1,
                limit: Some(1),
            }))
            .await
            .unwrap()
            .structured_content
            .unwrap();
        assert_eq!(second["offset"], 2);
        assert_eq!(second["text"], "x");
    }

    #[tokio::test]
    async fn diff_is_byte_paged_and_reports_capture_truncation() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        let mut run = test_run("diff", "succeeded", Vec::new());
        run.diff = "é".repeat(OUTPUT_PAGE_MAX);
        run.diff_truncated = true;
        server.state.lock().await.runs.push_back(run);

        let result = server
            .get_diff(Parameters(OutputRequest {
                run_id: "diff".into(),
                offset: 1,
                limit: Some(OUTPUT_PAGE_MAX * 2),
            }))
            .await
            .unwrap();
        let value = result.structured_content.unwrap();
        assert!(
            value["next_offset"].as_u64().unwrap() - value["offset"].as_u64().unwrap()
                <= OUTPUT_PAGE_MAX as u64
        );
        assert_eq!(value["eof"], false);
        assert_eq!(value["truncated"], true);
        assert!(value["text"].as_str().unwrap().starts_with('é'));
    }

    #[tokio::test]
    async fn diff_pagination_advances_past_a_multibyte_character() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        let mut run = test_run("unicode", "succeeded", Vec::new());
        run.diff = "éx".into();
        server.state.lock().await.runs.push_back(run);

        let result = server
            .get_diff(Parameters(OutputRequest {
                run_id: "unicode".into(),
                offset: 0,
                limit: Some(1),
            }))
            .await
            .unwrap();
        let value = result.structured_content.unwrap();
        assert!(value["next_offset"].as_u64().unwrap() > 0);
        assert_eq!(value["text"], "éx");
        assert_eq!(value["eof"], true);
    }

    #[tokio::test]
    async fn one_active_run_per_root_and_cancellation_are_enforced() {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let server = HkMcpServer::new(root.clone());
        let first = server
            .start(root.clone(), RunKind::SafeCheck)
            .await
            .unwrap();
        assert!(server.start(root, RunKind::SafeCheck).await.is_err());
        {
            let mut state = server.state.lock().await;
            let run = state
                .runs
                .iter_mut()
                .find(|run| run.id == first.id)
                .unwrap();
            run.status = "cancelling".into();
            run.cancel.cancel();
        }
        for _ in 0..100 {
            let snapshot = server.snapshot(&first.id).await.unwrap();
            if !matches!(
                snapshot.status.as_str(),
                "starting" | "running" | "cancelling"
            ) {
                assert_eq!(snapshot.status, "cancelled");
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("cancelled run did not complete");
    }

    #[cfg(debug_assertions)]
    #[tokio::test]
    async fn debug_shutdown_cancels_active_runs_before_exit() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        let run = test_run("active", "running", Vec::new());
        let cancellation = run.cancel.clone();
        server.state.lock().await.runs.push_back(run);

        assert_eq!(server.prepare_debug_shutdown().await, 1);
        assert!(cancellation.is_cancelled());
        assert_eq!(
            server.snapshot("active").await.unwrap().status,
            "cancelling"
        );
    }

    #[tokio::test]
    async fn protocol_initializes_and_lists_tools() {
        let root = tempfile::tempdir().unwrap();
        let server = HkMcpServer::new(root.path().canonicalize().unwrap());
        let (server_io, client_io) = tokio::io::duplex(64 * 1024);
        let server_task = tokio::spawn(async move {
            server
                .serve(server_io)
                .await
                .unwrap()
                .waiting()
                .await
                .unwrap();
        });
        let (read, mut write) = tokio::io::split(client_io);
        let mut read = BufReader::new(read);
        write
            .write_all(
                br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"hk-test","version":"1"}}}
"#,
            )
            .await
            .unwrap();
        let mut line = String::new();
        read.read_line(&mut line).await.unwrap();
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["serverInfo"]["name"], "hk");

        write
            .write_all(
                br#"{"jsonrpc":"2.0","method":"notifications/initialized"}
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
"#,
            )
            .await
            .unwrap();
        line.clear();
        read.read_line(&mut line).await.unwrap();
        let response: Value = serde_json::from_str(&line).unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        let expected_tool_count = if cfg!(debug_assertions) { 11 } else { 10 };
        assert_eq!(tools.len(), expected_tool_count);
        let tools = tools
            .iter()
            .map(|tool| (tool["name"].as_str().unwrap(), tool))
            .collect::<BTreeMap<_, _>>();
        let mut expected_tools = vec![
            "cancel_run",
            "get_diff",
            "get_output",
            "get_run",
            "inspect_project",
            "plan",
            "render_run",
            "start_check",
            "start_safe_check",
            "start_safe_fix",
        ];
        if cfg!(debug_assertions) {
            expected_tools.insert(1, "debug_shutdown");
        }
        assert_eq!(tools.keys().copied().collect::<Vec<_>>(), expected_tools,);
        assert_eq!(tools["start_check"]["annotations"]["destructiveHint"], true);
        assert_eq!(tools["start_check"]["annotations"]["openWorldHint"], true);
        assert_eq!(
            tools["start_safe_fix"]["annotations"]["destructiveHint"],
            false
        );
        assert_eq!(
            tools["inspect_project"]["annotations"]["readOnlyHint"],
            true
        );
        assert_eq!(
            tools["render_run"]["_meta"]["ui"]["resourceUri"],
            DASHBOARD_URI
        );
        assert_eq!(
            tools["render_run"]["_meta"]["openai/outputTemplate"],
            DASHBOARD_URI
        );

        write
            .write_all(
                br#"{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}
{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"ui://hk/run-dashboard"}}
"#,
            )
            .await
            .unwrap();
        line.clear();
        read.read_line(&mut line).await.unwrap();
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["result"]["resources"][0]["mimeType"], MCP_APP_MIME);
        line.clear();
        read.read_line(&mut line).await.unwrap();
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["result"]["contents"][0]["mimeType"], MCP_APP_MIME);
        assert!(
            response["result"]["contents"][0]["text"]
                .as_str()
                .unwrap()
                .contains("hk dashboard")
        );
        drop(write);
        server_task.abort();
        let _ = server_task.await;
    }
}
