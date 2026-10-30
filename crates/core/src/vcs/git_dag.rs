//! Autonomous Directed Acyclic Graph (DAG) Stages, Refactoring Locks, and Multi-Agent Git Collaboration
//!
//! Provides first-class software engineering coordination primitives for autonomous AI teams:
//! 1. **4-Stage DAG Execution Pipeline** (`PmScoping` -> `BlockingRefactorContractLock` -> `IncrementalParallelEdits` -> `ParentReviewMergeAndQa`).
//! 2. **Refactor Barrier (`RefactorLock`)**: Ensures that when an architectural refactor is underway, other agents await completion.
//! 3. **Sub-Agent Git Branching & 3-Way Merge (`GitBranchWorkflow`)**: Isolates subagent file operations into Git branches (`subagent/<agent_id>/<task_id>`)
//!    and merges non-overlapping edits safely via 3-way git merges (`git merge --no-ff`), flagging line conflicts for parent agent resolution.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{watch, Mutex};
use uuid::Uuid;

/// Policy validator for agent filesystem write access and role hierarchy.
pub trait AgentWriteAuthorizer: Send + Sync {
    fn is_parent_agent(&self, agent_id: &str) -> bool;
    fn check_write_permission(&self, agent_id: &str, rel_path: &str) -> Result<(), String>;
}

/// Default authorizer permitting standard hierarchical roles and prohibiting path traversal.
#[derive(Debug, Clone, Default)]
pub struct DefaultWriteAuthorizer;

impl AgentWriteAuthorizer for DefaultWriteAuthorizer {
    fn is_parent_agent(&self, agent_id: &str) -> bool {
        let clean = agent_id.trim_start_matches("agent:").to_lowercase();
        clean == "jenny" || clean == "orchestrator" || clean == "lead" || clean == "manager" || clean == "ceo"
    }

    fn check_write_permission(&self, _agent_id: &str, rel_path: &str) -> Result<(), String> {
        let clean = rel_path.replace('\\', "/");
        if clean.contains("..") {
            return Err("Path traversal ('..') is strictly prohibited".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DagStage {
    PmScoping,
    BlockingRefactorContractLock,
    IncrementalParallelEdits,
    ParentReviewMergeAndQa,
}

impl DagStage {
    pub fn step_number(&self) -> u8 {
        match self {
            DagStage::PmScoping => 1,
            DagStage::BlockingRefactorContractLock => 2,
            DagStage::IncrementalParallelEdits => 3,
            DagStage::ParentReviewMergeAndQa => 4,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            DagStage::PmScoping => "Stage 1: PM Scoping (Jenny)",
            DagStage::BlockingRefactorContractLock => {
                "Stage 2: Blocking Refactor / Contract Lock (Jane/John)"
            }
            DagStage::IncrementalParallelEdits => {
                "Stage 3: Incremental Parallel Edits (Sub-agents)"
            }
            DagStage::ParentReviewMergeAndQa => {
                "Stage 4: Parent Agent Review, Git Merge & QA Verification"
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DagStageTransition {
    pub stage: DagStage,
    pub label: String,
    pub entered_at: String,
    pub active_branch: String,
    pub base_commit_sha: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorLease {
    pub lease_id: String,
    pub holder_agent_id: String,
    pub reason: String,
    pub target_files: Vec<String>,
    pub acquired_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarrierWaitEvent {
    pub waiting_agent_id: String,
    pub blocked_by_agent_id: String,
    pub refactor_lease_id: String,
    pub refactor_reason: String,
    pub waited_ms: u64,
    pub resumed_at_commit_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileEdit {
    /// Writes full content to the relative file path on the sub-agent's branch.
    Write { path: String, content: String },
    /// Applies a targeted substring replacement relative to the branch's base commit file content.
    PatchReplace {
        path: String,
        old_text: String,
        new_text: String,
    },
}

impl FileEdit {
    pub fn path(&self) -> &str {
        match self {
            FileEdit::Write { path, .. } => path,
            FileEdit::PatchReplace { path, .. } => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PullRequestStatus {
    Open,
    Merged,
    Conflict,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentPullRequest {
    pub pr_id: String,
    pub author_agent_id: String,
    pub task_id: String,
    pub title: String,
    pub source_branch: String,
    pub target_branch: String,
    pub base_commit_sha: String,
    pub head_commit_sha: String,
    pub merge_commit_sha: Option<String>,
    pub modified_files: Vec<String>,
    pub unified_diff: String,
    pub status: PullRequestStatus,
    pub reviewed_by: Option<String>,
    pub conflict_files: Vec<String>,
    pub conflict_details: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MergeStatus {
    Merged,
    Conflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeResult {
    pub pr_id: String,
    pub status: MergeStatus,
    pub reviewer_agent_id: String,
    pub target_branch: String,
    pub merge_commit_sha: Option<String>,
    pub conflict_files: Vec<String>,
    pub conflict_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DagExecutionSummary {
    pub feature_branch: String,
    pub initial_base_commit: String,
    pub post_refactor_commit: String,
    pub final_head_commit: String,
    pub current_stage: Option<DagStage>,
    pub stage_history: Vec<DagStageTransition>,
    pub barrier_wait_events: Vec<BarrierWaitEvent>,
    pub pull_requests: Vec<SubAgentPullRequest>,
}

struct DagStateInner {
    current_stage: DagStage,
    feature_branch: String,
    initial_base_commit: String,
    stage3_base_commit: String,
    active_refactor: Option<RefactorLease>,
    stage_history: Vec<DagStageTransition>,
    barrier_wait_events: Vec<BarrierWaitEvent>,
    pull_requests: Vec<SubAgentPullRequest>,
}

/// Coordinates DAG stages, blocking refactor barriers, and real Git branch/PR/3-way merge workflows.
pub struct DagGitCoordinator {
    /// Isolated Git working repository used for real branch checkouts, commits, diffs, and 3-way merges.
    pub git_repo_path: PathBuf,
    /// Optional live client workspace path where merged `feat/<slug>` files are synchronized.
    pub workspace_sync_path: Option<PathBuf>,
    pub authorizer: Arc<dyn AgentWriteAuthorizer>,
    state: Mutex<DagStateInner>,
    /// Serializes raw `git` CLI invocations on `git_repo_path`.
    git_mutex: Mutex<()>,
    /// Broadcasts changes to the active `RefactorLease` (`Some(lease)` when locked, `None` when unlocked).
    refactor_watch_tx: watch::Sender<Option<RefactorLease>>,
    refactor_watch_rx: watch::Receiver<Option<RefactorLease>>,
}

impl std::fmt::Debug for DagGitCoordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DagGitCoordinator")
            .field("git_repo_path", &self.git_repo_path)
            .field("workspace_sync_path", &self.workspace_sync_path)
            .finish()
    }
}

impl DagGitCoordinator {
    /// Creates a coordinator backed by a dedicated Git sandbox mirrored from `workspace_path`.
    /// This avoids colliding with external `.git/index.lock` files in shared parent repositories
    /// while performing 100% real `git` branch, commit, diff, and 3-way `git merge --no-ff` operations.
    pub fn new_with_sandbox_and_authorizer(
        workspace_path: impl Into<PathBuf>,
        authorizer: Arc<dyn AgentWriteAuthorizer>,
    ) -> Result<Arc<Self>, String> {
        let ws_path: PathBuf = workspace_path.into();
        let sandbox_id = Uuid::new_v4().to_string()[..8].to_string();
        let sandbox_dir = std::env::temp_dir()
            .join("actualised_dag_git_sandboxes")
            .join(format!("repo_{}", sandbox_id));
        std::fs::create_dir_all(&sandbox_dir)
            .map_err(|e| format!("Failed to create DAG git sandbox dir: {}", e))?;

        Self::seed_sandbox_from_workspace(&ws_path, &sandbox_dir)?;
        Self::init_internal(sandbox_dir, Some(ws_path), authorizer)
    }

    pub fn new_with_sandbox_for_workspace(workspace_path: impl Into<PathBuf>) -> Result<Arc<Self>, String> {
        Self::new_with_sandbox_and_authorizer(workspace_path, Arc::new(DefaultWriteAuthorizer))
    }

    /// Creates a coordinator operating directly on `repo_path` as its Git repository.
    /// Ideal for isolated workspaces and deterministic unit/integration tests.
    pub fn new_in_repo(repo_path: impl Into<PathBuf>) -> Result<Arc<Self>, String> {
        let path: PathBuf = repo_path.into();
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create repo dir {}: {}", path.display(), e))?;
        Self::init_internal(path, None, Arc::new(DefaultWriteAuthorizer))
    }

    pub fn new_in_repo_with_authorizer(
        repo_path: impl Into<PathBuf>,
        authorizer: Arc<dyn AgentWriteAuthorizer>,
    ) -> Result<Arc<Self>, String> {
        let path: PathBuf = repo_path.into();
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create repo dir {}: {}", path.display(), e))?;
        Self::init_internal(path, None, authorizer)
    }

    fn init_internal(
        git_repo_path: PathBuf,
        workspace_sync_path: Option<PathBuf>,
        authorizer: Arc<dyn AgentWriteAuthorizer>,
    ) -> Result<Arc<Self>, String> {
        Self::ensure_git_repo_initialized(&git_repo_path)?;
        let head_sha = Self::run_git_raw(&git_repo_path, &["rev-parse", "HEAD"])?;

        let (tx, rx) = watch::channel(None);
        let initial_transition = DagStageTransition {
            stage: DagStage::PmScoping,
            label: DagStage::PmScoping.label().to_string(),
            entered_at: Utc::now().to_rfc3339(),
            active_branch: "main".to_string(),
            base_commit_sha: head_sha.clone(),
            note: "Initialized DAG Git Coordinator in Stage 1 (PM Scoping).".to_string(),
        };

        let inner = DagStateInner {
            current_stage: DagStage::PmScoping,
            feature_branch: "main".to_string(),
            initial_base_commit: head_sha.clone(),
            stage3_base_commit: head_sha,
            active_refactor: None,
            stage_history: vec![initial_transition],
            barrier_wait_events: Vec::new(),
            pull_requests: Vec::new(),
        };

        Ok(Arc::new(Self {
            git_repo_path,
            workspace_sync_path,
            authorizer,
            state: Mutex::new(inner),
            git_mutex: Mutex::new(()),
            refactor_watch_tx: tx,
            refactor_watch_rx: rx,
        }))
    }

    fn seed_sandbox_from_workspace(ws_path: &Path, sandbox_dir: &Path) -> Result<(), String> {
        let subdirs = ["specs", "resources", "dialogues", "scripts", "scenes", "shaders"];
        for sub in subdirs {
            let src = ws_path.join(sub);
            if src.exists() {
                let dst = sandbox_dir.join(sub);
                Self::copy_dir_recursive(&src, &dst)?;
            }
        }
        let proj = ws_path.join("project.godot");
        if proj.exists() {
            let _ = std::fs::copy(&proj, sandbox_dir.join("project.godot"));
        } else {
            let _ = std::fs::write(
                sandbox_dir.join("project.godot"),
                "; Engine configuration file.\nconfig_version=5\n",
            );
        }
        Ok(())
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        let entries = std::fs::read_dir(src).map_err(|e| e.to_string())?;
        for entry in entries.flatten() {
            let ty = entry.file_type().map_err(|e| e.to_string())?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if ty.is_dir() {
                Self::copy_dir_recursive(&src_path, &dst_path)?;
            } else if ty.is_file() {
                let _ = std::fs::copy(&src_path, &dst_path);
            }
        }
        Ok(())
    }

    fn ensure_git_repo_initialized(repo_path: &Path) -> Result<(), String> {
        let git_dir = repo_path.join(".git");
        if !git_dir.exists() {
            Self::run_git_raw(repo_path, &["init", "-b", "main"]).or_else(|_| {
                let res = Self::run_git_raw(repo_path, &["init"])?;
                let _ = Self::run_git_raw(repo_path, &["checkout", "-b", "main"]);
                Ok::<String, String>(res)
            })?;
        }
        Self::run_git_raw(repo_path, &["config", "user.name", "Playable Studio Coordinator"])?;
        Self::run_git_raw(repo_path, &["config", "user.email", "agents@actualised.ai"])?;
        // Disable GPG signing in sandbox commits
        let _ = Self::run_git_raw(repo_path, &["config", "commit.gpgsign", "false"]);

        // Ensure at least one initial commit exists on `main`
        if Self::run_git_raw(repo_path, &["rev-parse", "HEAD"]).is_err() {
            let readme = repo_path.join(".playable_workspace");
            if !readme.exists() {
                std::fs::write(&readme, "Playable DAG Git Workspace\n")
                    .map_err(|e| e.to_string())?;
            }
            Self::run_git_raw(repo_path, &["add", "-A"])?;
            Self::run_git_raw(
                repo_path,
                &["commit", "--allow-empty", "-m", "chore: initial studio workspace baseline"],
            )?;
            let _ = Self::run_git_raw(repo_path, &["branch", "-M", "main"]);
        }
        Ok(())
    }

    pub(crate) fn run_git_raw(repo_path: &Path, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute git {:?}: {}", args, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Err(format!(
                "git {:?} exited with {}: {}\n{}",
                args, output.status, stdout, stderr
            ))
        }
    }

    /// Like `run_git_raw` but returns stdout byte-exact (no trimming), for reading file contents
    /// so trailing newlines survive read -> edit -> write round trips.
    pub(crate) fn run_git_exact(repo_path: &Path, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute git {:?}: {}", args, e))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(format!(
                "git {:?} exited with {}: {}",
                args,
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    /// Commits a file on the active working tree with the specified commit message.
    pub async fn commit_file_on_working_tree(
        &self,
        clean_rel_path: &str,
        message: &str,
    ) -> Result<String, String> {
        let _git_guard = self.git_mutex.lock().await;
        Self::run_git_raw(&self.git_repo_path, &["add", clean_rel_path])?;
        let status = Self::run_git_raw(&self.git_repo_path, &["status", "--porcelain"])?;
        if status.is_empty() {
            return Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"]);
        }
        Self::run_git_raw(&self.git_repo_path, &["commit", "-m", message])?;
        let new_sha = Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"])?;
        let mut st = self.state.lock().await;
        st.stage3_base_commit = new_sha.clone();
        Ok(new_sha)
    }

    /// Stage 1 (`PmScoping`): Parent agent (`jenny`) initializes the build's target feature branch (`feat/<slug>`).
    pub async fn start_stage1_pm_scoping(
        &self,
        coordinator_agent_id: &str,
        feature_branch: &str,
        scoping_note: &str,
    ) -> Result<String, String> {
        if !self.authorizer.is_parent_agent(coordinator_agent_id) {
            return Err(format!(
                "Stage 1 (PM Scoping) must be initiated by parent agent 'jenny', not '{}'",
                coordinator_agent_id
            ));
        }

        let _git_guard = self.git_mutex.lock().await;
        let sanitized_branch = Self::sanitize_branch_name(feature_branch);
        // Checkout main or current HEAD and create/reset the feature branch
        let _ = Self::run_git_raw(&self.git_repo_path, &["checkout", "main"]);
        Self::run_git_raw(&self.git_repo_path, &["checkout", "-B", &sanitized_branch])?;
        let base_sha = Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"])?;

        let mut st = self.state.lock().await;
        st.current_stage = DagStage::PmScoping;
        st.feature_branch = sanitized_branch.clone();
        st.initial_base_commit = base_sha.clone();
        st.stage3_base_commit = base_sha.clone();
        st.pull_requests.clear();
        st.barrier_wait_events.clear();
        st.stage_history.push(DagStageTransition {
            stage: DagStage::PmScoping,
            label: DagStage::PmScoping.label().to_string(),
            entered_at: Utc::now().to_rfc3339(),
            active_branch: sanitized_branch,
            base_commit_sha: base_sha.clone(),
            note: scoping_note.to_string(),
        });

        Ok(base_sha)
    }

    /// Returns true if a major blocking refactor (`Stage 2`) is currently active.
    pub fn is_refactor_locked(&self) -> bool {
        self.refactor_watch_rx.borrow().is_some()
    }

    /// Returns the active `RefactorLease` if one is currently held.
    pub fn active_refactor_lease(&self) -> Option<RefactorLease> {
        self.refactor_watch_rx.borrow().clone()
    }

    /// Stage 2 (`BlockingRefactorContractLock`): Acquires an exclusive refactor lock so all other agents
    /// wait until the major refactor or interface contract update finishes and merges.
    pub async fn acquire_refactor_lock(
        &self,
        agent_id: &str,
        reason: &str,
        target_files: Vec<String>,
    ) -> Result<RefactorLease, String> {
        // Wait if another agent already holds a refactor lock
        self.wait_if_refactor_active(agent_id).await;

        let mut st = self.state.lock().await;
        if let Some(ref existing) = st.active_refactor {
            if existing.holder_agent_id != agent_id {
                return Err(format!(
                    "Refactor lock already held by '{}'",
                    existing.holder_agent_id
                ));
            }
            return Ok(existing.clone());
        }

        let lease = RefactorLease {
            lease_id: format!("ref_{}", &Uuid::new_v4().to_string()[..8]),
            holder_agent_id: agent_id.to_string(),
            reason: reason.to_string(),
            target_files,
            acquired_at: Utc::now().to_rfc3339(),
        };

        st.current_stage = DagStage::BlockingRefactorContractLock;
        st.active_refactor = Some(lease.clone());
        let branch = st.feature_branch.clone();
        let base_sha = st.stage3_base_commit.clone();
        st.stage_history.push(DagStageTransition {
            stage: DagStage::BlockingRefactorContractLock,
            label: DagStage::BlockingRefactorContractLock.label().to_string(),
            entered_at: Utc::now().to_rfc3339(),
            active_branch: branch,
            base_commit_sha: base_sha,
            note: format!("Agent '{}' acquired RefactorLock: {}", agent_id, reason),
        });

        let _ = self.refactor_watch_tx.send(Some(lease.clone()));
        Ok(lease)
    }

    /// Completes the active Stage 2 blocking refactor, commits & merges the refactored files onto the
    /// feature branch, updates `stage3_base_commit` to the new post-refactor SHA, and releases the barrier
    /// so waiting Stage 3 sub-agents wake up and branch from the post-refactor state.
    pub async fn complete_refactor_and_unlock(
        &self,
        agent_id: &str,
        edits: Vec<FileEdit>,
        commit_message: &str,
    ) -> Result<String, String> {
        {
            let st = self.state.lock().await;
            match &st.active_refactor {
                Some(lease) if lease.holder_agent_id == agent_id => {}
                Some(lease) => {
                    return Err(format!(
                        "Agent '{}' cannot release refactor lock held by '{}'",
                        agent_id, lease.holder_agent_id
                    ));
                }
                None => {
                    return Err("No active refactor lock to complete".to_string());
                }
            }
        }

        for edit in &edits {
            if let Err(e) = self.authorizer.check_write_permission(agent_id, edit.path()) {
                let _ = self.release_refactor_lock_on_error(agent_id).await;
                return Err(e);
            }
        }

        let new_base_sha = {
            let _git_guard = self.git_mutex.lock().await;
            let (feature_branch, base_commit) = {
                let st = self.state.lock().await;
                (st.feature_branch.clone(), st.stage3_base_commit.clone())
            };

            let refactor_branch = format!(
                "refactor/{}/{}",
                Self::sanitize_segment(agent_id),
                &Uuid::new_v4().to_string()[..6]
            );
            Self::run_git_raw(
                &self.git_repo_path,
                &["checkout", "-B", &refactor_branch, &base_commit],
            )?;

            for edit in &edits {
                self.apply_file_edit_in_worktree(edit)?;
            }

            Self::run_git_raw(&self.git_repo_path, &["add", "-A"])?;
            Self::run_git_raw(
                &self.git_repo_path,
                &["commit", "--allow-empty", "-m", commit_message],
            )?;

            Self::run_git_raw(&self.git_repo_path, &["checkout", &feature_branch])?;
            Self::run_git_raw(
                &self.git_repo_path,
                &[
                    "merge",
                    "--no-ff",
                    "-m",
                    &format!("Merge major refactor from {}: {}", refactor_branch, commit_message),
                    &refactor_branch,
                ],
            )?;

            let sha = Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"])?;
            self.sync_files_to_workspace(&edits)?;
            sha
        };

        {
            let mut st = self.state.lock().await;
            st.active_refactor = None;
            st.stage3_base_commit = new_base_sha.clone();
            st.current_stage = DagStage::IncrementalParallelEdits;
            let branch = st.feature_branch.clone();
            st.stage_history.push(DagStageTransition {
                stage: DagStage::IncrementalParallelEdits,
                label: DagStage::IncrementalParallelEdits.label().to_string(),
                entered_at: Utc::now().to_rfc3339(),
                active_branch: branch,
                base_commit_sha: new_base_sha.clone(),
                note: format!(
                    "Refactor by '{}' merged ({}); Stage 3 parallel sub-agent edits unlocked.",
                    agent_id,
                    &new_base_sha[..8.min(new_base_sha.len())]
                ),
            });
        }

        let _ = self.refactor_watch_tx.send(None);
        Ok(new_base_sha)
    }

    async fn release_refactor_lock_on_error(&self, agent_id: &str) -> Result<(), String> {
        let mut st = self.state.lock().await;
        if st
            .active_refactor
            .as_ref()
            .map(|l| l.holder_agent_id == agent_id)
            .unwrap_or(false)
        {
            st.active_refactor = None;
            let _ = self.refactor_watch_tx.send(None);
        }
        Ok(())
    }

    /// Blocks the caller if another agent currently holds the `RefactorLock`, recording a `BarrierWaitEvent`
    /// with the exact wait duration and post-refactor commit SHA once released.
    pub async fn wait_if_refactor_active(&self, waiting_agent_id: &str) -> Option<BarrierWaitEvent> {
        let mut rx = self.refactor_watch_rx.clone();
        let initial_lease = rx.borrow().clone();
        let lease = match initial_lease {
            Some(l) if l.holder_agent_id != waiting_agent_id => l,
            _ => return None,
        };

        let start = Instant::now();
        loop {
            if rx.borrow().is_none() {
                break;
            }
            if rx.changed().await.is_err() {
                break;
            }
        }
        let waited_ms = start.elapsed().as_millis() as u64;

        let mut st = self.state.lock().await;
        let event = BarrierWaitEvent {
            waiting_agent_id: waiting_agent_id.to_string(),
            blocked_by_agent_id: lease.holder_agent_id,
            refactor_lease_id: lease.lease_id,
            refactor_reason: lease.reason,
            waited_ms,
            resumed_at_commit_sha: st.stage3_base_commit.clone(),
        };
        st.barrier_wait_events.push(event.clone());
        Some(event)
    }

    /// Reads the current Stage 3 base commit SHA on the feature branch.
    pub async fn current_base_commit(&self) -> String {
        let st = self.state.lock().await;
        st.stage3_base_commit.clone()
    }

    /// Reads a file from the Git repository at the current Stage 3 base commit.
    pub async fn read_file_at_base(&self, rel_path: &str) -> Result<String, String> {
        let clean = Self::clean_rel_path(rel_path)?;
        let base_sha = self.current_base_commit().await;
        let _git_guard = self.git_mutex.lock().await;
        Self::run_git_exact(&self.git_repo_path, &["show", &format!("{}:{}", base_sha, clean)])
    }

    /// Reads a file from the current tip of the active feature branch (`feat/<slug>`).
    pub async fn read_file_on_feature_branch(&self, rel_path: &str) -> Result<String, String> {
        let clean = Self::clean_rel_path(rel_path)?;
        let feature_branch = {
            let st = self.state.lock().await;
            st.feature_branch.clone()
        };
        let _git_guard = self.git_mutex.lock().await;
        Self::run_git_exact(
            &self.git_repo_path,
            &["show", &format!("{}:{}", feature_branch, clean)],
        )
    }

    /// Stage 3 (`IncrementalParallelEdits`): Sub-agent checks out a dedicated branch
    /// (`subagent/<agent_id>/<task_id>`) from the Stage 3 base commit (or `explicit_base_commit`),
    /// commits its file edits, and opens a `SubAgentPullRequest` for the parent agent (`jenny`) to review and merge.
    ///
    /// Enforces:
    /// 1. Parent agent (`jenny` / `orchestrator`) cannot open sub-agent branches/PRs.
    /// 2. Sub-agent waits if a Stage 2 `RefactorLock` is currently active.
    /// 3. All modified paths pass `FileSystemJail::check_write_permission`.
    pub async fn submit_subagent_branch_pr(
        &self,
        agent_id: &str,
        task_id: &str,
        title: &str,
        edits: Vec<FileEdit>,
        explicit_base_commit: Option<&str>,
    ) -> Result<SubAgentPullRequest, String> {
        if self.authorizer.is_parent_agent(agent_id) {
            return Err(format!(
                "Role Violation: Parent agent '{}' may only review and merge PRs; only sub-agents may check out branches and open PRs.",
                agent_id
            ));
        }

        if edits.is_empty() {
            return Err("Sub-agent PR must contain at least one file edit".to_string());
        }

        for edit in &edits {
            if let Err(e) = self.authorizer.check_write_permission(agent_id, edit.path()) {
                return Err(e);
            }
        }

        // Wait if another agent is currently performing a Stage 2 blocking refactor!
        let waited_on_refactor = self.wait_if_refactor_active(agent_id).await.is_some();

        let _git_guard = self.git_mutex.lock().await;
        let (feature_branch, stage3_base) = {
            let mut st = self.state.lock().await;
            if st.current_stage != DagStage::IncrementalParallelEdits {
                st.current_stage = DagStage::IncrementalParallelEdits;
                let fb = st.feature_branch.clone();
                let base = st.stage3_base_commit.clone();
                st.stage_history.push(DagStageTransition {
                    stage: DagStage::IncrementalParallelEdits,
                    label: DagStage::IncrementalParallelEdits.label().to_string(),
                    entered_at: Utc::now().to_rfc3339(),
                    active_branch: fb,
                    base_commit_sha: base,
                    note: format!("Sub-agent '{}' entered Stage 3 parallel edits.", agent_id),
                });
            }
            (st.feature_branch.clone(), st.stage3_base_commit.clone())
        };

        // If the sub-agent was blocked by a major refactor, always rebase its starting point onto
        // the post-refactor baseline (`stage3_base`) so it includes the refactored contracts.
        let base_commit = if waited_on_refactor {
            stage3_base
        } else {
            explicit_base_commit
                .map(str::to_string)
                .unwrap_or(stage3_base)
        };

        let source_branch = format!(
            "subagent/{}/{}",
            Self::sanitize_segment(agent_id),
            Self::sanitize_segment(task_id)
        );

        // Checkout the sub-agent branch rooted at `base_commit`
        Self::run_git_raw(
            &self.git_repo_path,
            &["checkout", "-B", &source_branch, &base_commit],
        )?;

        let mut modified_files = Vec::new();
        for edit in &edits {
            let rel = self.apply_file_edit_in_worktree(edit)?;
            if !modified_files.contains(&rel) {
                modified_files.push(rel);
            }
        }

        Self::run_git_raw(&self.git_repo_path, &["add", "-A"])?;
        let commit_msg = format!("feat({}): {} [{}]", agent_id, title, task_id);
        Self::run_git_raw(
            &self.git_repo_path,
            &["commit", "--allow-empty", "-m", &commit_msg],
        )?;

        let head_commit = Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"])?;
        let unified_diff = Self::run_git_raw(
            &self.git_repo_path,
            &["diff", &format!("{}..{}", base_commit, head_commit)],
        )
        .unwrap_or_default();

        // Return working tree back to the target feature branch
        Self::run_git_raw(&self.git_repo_path, &["checkout", &feature_branch])?;

        let now = Utc::now().to_rfc3339();
        let mut st = self.state.lock().await;
        let pr_num = st.pull_requests.len() + 1;
        let pr = SubAgentPullRequest {
            pr_id: format!("PR-{}", pr_num),
            author_agent_id: agent_id.to_string(),
            task_id: task_id.to_string(),
            title: title.to_string(),
            source_branch,
            target_branch: feature_branch,
            base_commit_sha: base_commit,
            head_commit_sha: head_commit,
            merge_commit_sha: None,
            modified_files,
            unified_diff,
            status: PullRequestStatus::Open,
            reviewed_by: None,
            conflict_files: Vec::new(),
            conflict_details: None,
            created_at: now.clone(),
            updated_at: now,
        };
        st.pull_requests.push(pr.clone());
        Ok(pr)
    }

    /// Stage 4 (`ParentReviewMergeAndQa`): Parent agent (`jenny` / `orchestrator`) reviews a sub-agent's
    /// pull request and performs a 3-way Git merge (`git merge --no-ff`) into `feat/<slug>`.
    ///
    /// - If two sub-agents edited different sections of the same file from the same base commit,
    ///   Git's 3-way merge cleanly combines both edits without losing either change.
    /// - If two sub-agents edited the exact same lines with conflicting values, Git detects a
    ///   merge conflict, records the conflict markers and conflicting files on the PR, aborts the
    ///   in-progress merge to keep `feat/<slug>` clean, and flags the PR as `PullRequestStatus::Conflict`.
    pub async fn review_and_merge_pr(
        &self,
        reviewer_agent_id: &str,
        pr_id: &str,
    ) -> Result<MergeResult, String> {
        if !self.authorizer.is_parent_agent(reviewer_agent_id) {
            return Err(format!(
                "Role Violation: Only the parent agent ('jenny') may review and merge pull requests (attempted by '{}').",
                reviewer_agent_id
            ));
        }

        let _git_guard = self.git_mutex.lock().await;
        let (source_branch, target_branch, modified_files) = {
            let mut st = self.state.lock().await;
            if st.current_stage != DagStage::ParentReviewMergeAndQa {
                st.current_stage = DagStage::ParentReviewMergeAndQa;
                let fb = st.feature_branch.clone();
                let base = st.stage3_base_commit.clone();
                st.stage_history.push(DagStageTransition {
                    stage: DagStage::ParentReviewMergeAndQa,
                    label: DagStage::ParentReviewMergeAndQa.label().to_string(),
                    entered_at: Utc::now().to_rfc3339(),
                    active_branch: fb,
                    base_commit_sha: base,
                    note: format!(
                        "Parent agent '{}' entered Stage 4 PR review & 3-way Git merge.",
                        reviewer_agent_id
                    ),
                });
            }

            let pr = st
                .pull_requests
                .iter()
                .find(|p| p.pr_id == pr_id)
                .ok_or_else(|| format!("Pull request '{}' not found", pr_id))?;

            if pr.status == PullRequestStatus::Merged {
                return Ok(MergeResult {
                    pr_id: pr.pr_id.clone(),
                    status: MergeStatus::Merged,
                    reviewer_agent_id: reviewer_agent_id.to_string(),
                    target_branch: pr.target_branch.clone(),
                    merge_commit_sha: pr.merge_commit_sha.clone(),
                    conflict_files: Vec::new(),
                    conflict_details: None,
                });
            }

            (
                pr.source_branch.clone(),
                pr.target_branch.clone(),
                pr.modified_files.clone(),
            )
        };

        Self::run_git_raw(&self.git_repo_path, &["checkout", &target_branch])?;

        let merge_msg = format!(
            "Merge {} ({}) into {} [Reviewed-by: {}]",
            pr_id, source_branch, target_branch, reviewer_agent_id
        );
        let merge_output = Command::new("git")
            .current_dir(&self.git_repo_path)
            .args(["merge", "--no-ff", "-m", &merge_msg, &source_branch])
            .output()
            .map_err(|e| format!("Failed to invoke git merge: {}", e))?;

        let now = Utc::now().to_rfc3339();
        if merge_output.status.success() {
            let merge_sha = Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"])?;
            self.sync_rel_paths_to_workspace(&modified_files)?;

            let mut st = self.state.lock().await;
            if let Some(pr) = st.pull_requests.iter_mut().find(|p| p.pr_id == pr_id) {
                pr.status = PullRequestStatus::Merged;
                pr.reviewed_by = Some(reviewer_agent_id.to_string());
                pr.merge_commit_sha = Some(merge_sha.clone());
                pr.updated_at = now;
            }

            Ok(MergeResult {
                pr_id: pr_id.to_string(),
                status: MergeStatus::Merged,
                reviewer_agent_id: reviewer_agent_id.to_string(),
                target_branch,
                merge_commit_sha: Some(merge_sha),
                conflict_files: Vec::new(),
                conflict_details: None,
            })
        } else {
            // Capture unmerged / conflicting files and conflict markers before aborting
            let unmerged_raw = Self::run_git_raw(
                &self.git_repo_path,
                &["diff", "--name-only", "--diff-filter=U"],
            )
            .unwrap_or_default();
            let mut conflict_files: Vec<String> = unmerged_raw
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if conflict_files.is_empty() {
                conflict_files = modified_files.clone();
            }

            let mut conflict_snippets = Vec::new();
            for cf in &conflict_files {
                let full_cf = self.git_repo_path.join(cf);
                if let Ok(content) = std::fs::read_to_string(&full_cf) {
                    conflict_snippets.push(format!("--- Conflict in {} ---\n{}", cf, content));
                }
            }
            let conflict_details = if conflict_snippets.is_empty() {
                format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&merge_output.stdout).trim(),
                    String::from_utf8_lossy(&merge_output.stderr).trim()
                )
            } else {
                conflict_snippets.join("\n")
            };

            // Abort the conflicted merge so target_branch remains clean
            let _ = Self::run_git_raw(&self.git_repo_path, &["merge", "--abort"]);
            let _ = Self::run_git_raw(&self.git_repo_path, &["reset", "--hard", "HEAD"]);

            let mut st = self.state.lock().await;
            if let Some(pr) = st.pull_requests.iter_mut().find(|p| p.pr_id == pr_id) {
                pr.status = PullRequestStatus::Conflict;
                pr.reviewed_by = Some(reviewer_agent_id.to_string());
                pr.conflict_files = conflict_files.clone();
                pr.conflict_details = Some(conflict_details.clone());
                pr.updated_at = now;
            }

            Ok(MergeResult {
                pr_id: pr_id.to_string(),
                status: MergeStatus::Conflict,
                reviewer_agent_id: reviewer_agent_id.to_string(),
                target_branch,
                merge_commit_sha: None,
                conflict_files,
                conflict_details: Some(conflict_details),
            })
        }
    }

    /// Reviews and merges all currently open sub-agent PRs in FIFO order using the parent agent (`jenny`).
    pub async fn review_and_merge_all_open_prs(
        &self,
        reviewer_agent_id: &str,
    ) -> Result<Vec<MergeResult>, String> {
        let open_ids: Vec<String> = {
            let st = self.state.lock().await;
            st.pull_requests
                .iter()
                .filter(|p| p.status == PullRequestStatus::Open)
                .map(|p| p.pr_id.clone())
                .collect()
        };

        let mut results = Vec::with_capacity(open_ids.len());
        for pr_id in open_ids {
            results.push(self.review_and_merge_pr(reviewer_agent_id, &pr_id).await?);
        }
        Ok(results)
    }

    /// Allows the parent agent (`jenny`) to resolve a `PullRequestStatus::Conflict` PR by supplying
    /// the resolved file contents and committing the merge resolution onto `feat/<slug>`.
    pub async fn resolve_conflict_by_parent(
        &self,
        reviewer_agent_id: &str,
        pr_id: &str,
        resolved_files: HashMap<String, String>,
    ) -> Result<MergeResult, String> {
        if !self.authorizer.is_parent_agent(reviewer_agent_id) {
            return Err(format!(
                "Role Violation: Only parent agent ('jenny') may resolve PR conflicts (attempted by '{}').",
                reviewer_agent_id
            ));
        }

        let _git_guard = self.git_mutex.lock().await;
        let (source_branch, target_branch) = {
            let st = self.state.lock().await;
            let pr = st
                .pull_requests
                .iter()
                .find(|p| p.pr_id == pr_id)
                .ok_or_else(|| format!("Pull request '{}' not found", pr_id))?;
            if pr.status != PullRequestStatus::Conflict {
                return Err(format!(
                    "Pull request '{}' is not in Conflict status (current: {:?})",
                    pr_id, pr.status
                ));
            }
            (pr.source_branch.clone(), pr.target_branch.clone())
        };

        Self::run_git_raw(&self.git_repo_path, &["checkout", &target_branch])?;
        // Start merge without committing (expected to conflict, then we overwrite with parent resolution)
        let _ = Command::new("git")
            .current_dir(&self.git_repo_path)
            .args(["merge", "--no-commit", "--no-ff", &source_branch])
            .output();

        let mut touched_paths = Vec::new();
        for (rel_path, content) in &resolved_files {
            let clean = Self::clean_rel_path(rel_path)?;
            let full = self.git_repo_path.join(&clean);
            if let Some(parent) = full.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            std::fs::write(&full, content).map_err(|e| e.to_string())?;
            touched_paths.push(clean);
        }

        Self::run_git_raw(&self.git_repo_path, &["add", "-A"])?;
        let msg = format!(
            "Merge & resolve conflict for {} ({}) by parent {}",
            pr_id, source_branch, reviewer_agent_id
        );
        Self::run_git_raw(&self.git_repo_path, &["commit", "-m", &msg])?;
        let merge_sha = Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"])?;

        self.sync_rel_paths_to_workspace(&touched_paths)?;

        let mut st = self.state.lock().await;
        if let Some(pr) = st.pull_requests.iter_mut().find(|p| p.pr_id == pr_id) {
            pr.status = PullRequestStatus::Merged;
            pr.reviewed_by = Some(reviewer_agent_id.to_string());
            pr.merge_commit_sha = Some(merge_sha.clone());
            pr.updated_at = Utc::now().to_rfc3339();
        }

        Ok(MergeResult {
            pr_id: pr_id.to_string(),
            status: MergeStatus::Merged,
            reviewer_agent_id: reviewer_agent_id.to_string(),
            target_branch,
            merge_commit_sha: Some(merge_sha),
            conflict_files: Vec::new(),
            conflict_details: None,
        })
    }

    /// Returns a snapshot of the full DAG stage history, refactor barrier wait events, and sub-agent PRs.
    pub async fn summary(&self) -> DagExecutionSummary {
        let head_sha = {
            let _git_guard = self.git_mutex.lock().await;
            Self::run_git_raw(&self.git_repo_path, &["rev-parse", "HEAD"]).unwrap_or_default()
        };
        let st = self.state.lock().await;
        DagExecutionSummary {
            feature_branch: st.feature_branch.clone(),
            initial_base_commit: st.initial_base_commit.clone(),
            post_refactor_commit: st.stage3_base_commit.clone(),
            final_head_commit: head_sha,
            current_stage: Some(st.current_stage),
            stage_history: st.stage_history.clone(),
            barrier_wait_events: st.barrier_wait_events.clone(),
            pull_requests: st.pull_requests.clone(),
        }
    }

    fn apply_file_edit_in_worktree(&self, edit: &FileEdit) -> Result<String, String> {
        match edit {
            FileEdit::Write { path, content } => {
                let clean = Self::clean_rel_path(path)?;
                let full = self.git_repo_path.join(&clean);
                if let Some(parent) = full.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&full, content).map_err(|e| e.to_string())?;
                Ok(clean)
            }
            FileEdit::PatchReplace {
                path,
                old_text,
                new_text,
            } => {
                let clean = Self::clean_rel_path(path)?;
                let full = self.git_repo_path.join(&clean);
                let existing = std::fs::read_to_string(&full).map_err(|e| {
                    format!("Failed to read {} for PatchReplace: {}", clean, e)
                })?;
                if !existing.contains(old_text) {
                    return Err(format!(
                        "PatchReplace target text not found in '{}'",
                        clean
                    ));
                }
                let updated = existing.replacen(old_text, new_text, 1);
                std::fs::write(&full, updated).map_err(|e| e.to_string())?;
                Ok(clean)
            }
        }
    }

    fn sync_files_to_workspace(&self, edits: &[FileEdit]) -> Result<(), String> {
        let paths: Result<Vec<String>, String> =
            edits.iter().map(|e| Self::clean_rel_path(e.path())).collect();
        self.sync_rel_paths_to_workspace(&paths?)
    }

    fn sync_rel_paths_to_workspace(&self, rel_paths: &[String]) -> Result<(), String> {
        let Some(ref ws_root) = self.workspace_sync_path else {
            return Ok(());
        };
        for rel in rel_paths {
            let clean = Self::clean_rel_path(rel)?;
            let src = self.git_repo_path.join(&clean);
            let dst = ws_root.join(&clean);
            if src.exists() {
                if let Some(parent) = dst.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::copy(&src, &dst);
            }
        }
        Ok(())
    }

    fn clean_rel_path(rel_path: &str) -> Result<String, String> {
        let clean = super::workspace::normalize_canonical_path(rel_path);
        if clean.is_empty() || clean.contains("..") {
            return Err("Invalid or traversal file path".to_string());
        }
        Ok(clean)
    }

    fn sanitize_branch_name(branch: &str) -> String {
        let trimmed = branch.trim().trim_start_matches('/');
        if trimmed.is_empty() {
            "feat/live-update".to_string()
        } else {
            trimmed
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '/' || c == '-' || c == '_' {
                        c
                    } else {
                        '-'
                    }
                })
                .collect()
        }
    }

    fn sanitize_segment(seg: &str) -> String {
        let s: String = seg
            .trim()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect();
        if s.is_empty() {
            "task".to_string()
        } else {
            s.to_lowercase()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_temp_repo(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("actualised_dag_git_tests")
            .join(format!("{}_{}", prefix, Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn test_blocking_refactor_barrier_makes_subagents_wait() {
        let repo_dir = make_temp_repo("refactor_barrier");
        let coordinator = DagGitCoordinator::new_in_repo(&repo_dir).unwrap();

        // Stage 1: Jenny initializes feature branch `feat/warden-refactor`
        let initial_sha = coordinator
            .start_stage1_pm_scoping(
                "jenny",
                "feat/warden-refactor",
                "Scope major contract refactor before parallel sub-agent edits",
            )
            .await
            .unwrap();

        // Stage 2: Jane acquires a blocking RefactorLock (`is_major_refactor`)
        let lease = coordinator
            .acquire_refactor_lock(
                "jane",
                "Major refactor of specs/interface_contracts.json and base arena controller",
                vec!["specs/interface_contracts.json".to_string()],
            )
            .await
            .unwrap();
        assert!(coordinator.is_refactor_locked());
        assert_eq!(lease.holder_agent_id, "jane");

        // Spawn two sub-agents (`john` and `geoff`) that try to submit PRs while `jane`'s refactor is in progress
        let coord_john = Arc::clone(&coordinator);
        let john_handle = tokio::spawn(async move {
            coord_john
                .submit_subagent_branch_pr(
                    "john",
                    "gdd-update",
                    "Add molten boss stats to gdd_design.json",
                    vec![FileEdit::Write {
                        path: "specs/gdd_design.json".to_string(),
                        content: "{\"boss_name\": \"Magma Warden\"}\n".to_string(),
                    }],
                    None,
                )
                .await
        });

        let coord_geoff = Arc::clone(&coordinator);
        let geoff_handle = tokio::spawn(async move {
            coord_geoff
                .submit_subagent_branch_pr(
                    "geoff",
                    "terrain-seed",
                    "Update terrain graph resource",
                    vec![FileEdit::Write {
                        path: "resources/terrain_graphs/fire_ocean_rig.tres".to_string(),
                        content: "[gd_resource type=\"Resource\" format=3]\n[resource]\nresource_name = \"Molten Rig\"\n".to_string(),
                    }],
                    None,
                )
                .await
        });

        // Give spawned tasks time to hit the refactor barrier and block
        tokio::time::sleep(Duration::from_millis(90)).await;

        // Complete Jane's refactor and unlock Stage 3
        let post_refactor_sha = coordinator
            .complete_refactor_and_unlock(
                "jane",
                vec![FileEdit::Write {
                    path: "specs/interface_contracts.json".to_string(),
                    content: "{\n  \"version\": 2,\n  \"target_scene\": \"res://scenes/zones/fire_prison_rig.tscn\"\n}\n".to_string(),
                }],
                "refactor(jane): upgrade interface_contracts.json to v2 schema",
            )
            .await
            .unwrap();

        assert!(!coordinator.is_refactor_locked());
        assert_ne!(initial_sha, post_refactor_sha);

        // Both waiting sub-agents now complete and open their PRs branched from `post_refactor_sha`
        let john_pr = john_handle.await.unwrap().unwrap();
        let geoff_pr = geoff_handle.await.unwrap().unwrap();

        assert_eq!(john_pr.base_commit_sha, post_refactor_sha);
        assert_eq!(geoff_pr.base_commit_sha, post_refactor_sha);

        let summary = coordinator.summary().await;
        assert_eq!(summary.barrier_wait_events.len(), 2);
        for ev in &summary.barrier_wait_events {
            assert_eq!(ev.blocked_by_agent_id, "jane");
            assert!(
                ev.waited_ms >= 50,
                "Expected sub-agent {} to wait >= 50ms on refactor lock, waited {}ms",
                ev.waiting_agent_id,
                ev.waited_ms
            );
            assert_eq!(ev.resumed_at_commit_sha, post_refactor_sha);
        }

        // Stage 4: Parent agent Jenny merges both sub-agent PRs
        let merge_results = coordinator
            .review_and_merge_all_open_prs("jenny")
            .await
            .unwrap();
        assert_eq!(merge_results.len(), 2);
        assert!(merge_results.iter().all(|r| r.status == MergeStatus::Merged));
    }

    #[tokio::test]
    async fn test_two_subagents_concurrent_edits_same_file_clean_3way_merge() {
        let repo_dir = make_temp_repo("clean_3way_merge");
        let base_script = concat!(
            "extends Node3D\n",
            "class_name FirePrisonController\n",
            "\n",
            "# --- SECTION A: BOSS TUNING ---\n",
            "@export var boss_name: String = \"Fire Warden Boss\"\n",
            "@export var boss_max_hp: int = 2400\n",
            "@export var primary_ability_damage: int = 225\n",
            "\n",
            "# --- SECTION B: ARENA CONSTANTS ---\n",
            "const TICK_RATE_SEC: float = 0.033\n",
            "const MAX_PLAYERS: int = 8\n",
            "const ARENA_GRAVITY: float = 9.8\n",
            "\n",
            "# --- SECTION C: HAZARD CONFIGURATION ---\n",
            "@export var hazard_name: String = \"Volcanic Shockwave Ring\"\n",
            "@export var hazard_radius: float = 95.0\n",
            "@export var hazard_interval_sec: float = 6.0\n"
        );

        let script_rel = "scripts/zones/fire_prison_controller.gd";
        let script_full = repo_dir.join(script_rel);
        std::fs::create_dir_all(script_full.parent().unwrap()).unwrap();
        std::fs::write(&script_full, base_script).unwrap();

        let coordinator = DagGitCoordinator::new_in_repo(&repo_dir).unwrap();
        let base_sha = coordinator
            .start_stage1_pm_scoping(
                "jenny",
                "feat/concurrent-controller-edits",
                "Initialize feature branch for concurrent GDScript edits",
            )
            .await
            .unwrap();

        // Both sub-agents (`jane/combat` and `jane/hazards`) read the SAME initial file at `base_sha`
        let snapshot_for_combat = coordinator.read_file_at_base(script_rel).await.unwrap();
        let snapshot_for_hazards = coordinator.read_file_at_base(script_rel).await.unwrap();
        assert_eq!(snapshot_for_combat, base_script);
        assert_eq!(snapshot_for_hazards, base_script);

        // Sub-agent 1 (`jane/combat`) edits SECTION A (top of file) and writes the full file on its branch
        let combat_edited = snapshot_for_combat
            .replace(
                "@export var boss_max_hp: int = 2400",
                "@export var boss_max_hp: int = 4200",
            )
            .replace(
                "@export var primary_ability_damage: int = 225",
                "@export var primary_ability_damage: int = 380",
            );

        // Sub-agent 2 (`jane/hazards`) concurrently edits SECTION C (bottom of file) from the original snapshot!
        let hazards_edited = snapshot_for_hazards
            .replace(
                "@export var hazard_name: String = \"Volcanic Shockwave Ring\"",
                "@export var hazard_name: String = \"Molten Obsidian Geyser\"",
            )
            .replace(
                "@export var hazard_radius: float = 95.0",
                "@export var hazard_radius: float = 135.0",
            );

        // Verify role enforcement: parent agent `jenny` is NOT allowed to open a sub-agent branch/PR
        let jenny_pr_err = coordinator
            .submit_subagent_branch_pr(
                "jenny",
                "illegal-parent-pr",
                "Parent trying to open subagent PR",
                vec![FileEdit::Write {
                    path: "specs/project_board.json".to_string(),
                    content: "{}".to_string(),
                }],
                Some(&base_sha),
            )
            .await;
        assert!(jenny_pr_err.is_err());

        // Both sub-agents open their PRs against `base_sha`
        let pr_combat = coordinator
            .submit_subagent_branch_pr(
                "jane/combat",
                "tune-boss-hp",
                "Increase boss HP to 4200 and ability damage to 380",
                vec![FileEdit::Write {
                    path: script_rel.to_string(),
                    content: combat_edited,
                }],
                Some(&base_sha),
            )
            .await
            .unwrap();

        let pr_hazards = coordinator
            .submit_subagent_branch_pr(
                "jane/hazards",
                "tune-lava-geyser",
                "Switch hazard to Molten Obsidian Geyser with 135m radius",
                vec![FileEdit::Write {
                    path: script_rel.to_string(),
                    content: hazards_edited,
                }],
                Some(&base_sha),
            )
            .await
            .unwrap();

        assert_eq!(pr_combat.source_branch, "subagent/jane-combat/tune-boss-hp");
        assert_eq!(pr_hazards.source_branch, "subagent/jane-hazards/tune-lava-geyser");
        assert!(pr_combat.unified_diff.contains("boss_max_hp: int = 4200"));
        assert!(pr_hazards.unified_diff.contains("Molten Obsidian Geyser"));

        // Verify role enforcement: sub-agent cannot merge PRs; only parent agent `jenny` can
        let illegal_merge = coordinator
            .review_and_merge_pr("jane/combat", &pr_combat.pr_id)
            .await;
        assert!(illegal_merge.is_err());

        // Parent agent `jenny` reviews and merges PR 1 (`pr_combat`) and PR 2 (`pr_hazards`)
        let res1 = coordinator
            .review_and_merge_pr("jenny", &pr_combat.pr_id)
            .await
            .unwrap();
        assert_eq!(res1.status, MergeStatus::Merged);

        let res2 = coordinator
            .review_and_merge_pr("jenny", &pr_hazards.pr_id)
            .await
            .unwrap();
        assert_eq!(res2.status, MergeStatus::Merged);

        // Verify the merged file on `feat/concurrent-controller-edits` preserves BOTH sub-agents' changes!
        let merged_content = coordinator
            .read_file_on_feature_branch(script_rel)
            .await
            .unwrap();
        assert!(
            merged_content.contains("@export var boss_max_hp: int = 4200"),
            "Merged file lost jane/combat's boss_max_hp edit:\n{}",
            merged_content
        );
        assert!(
            merged_content.contains("@export var primary_ability_damage: int = 380"),
            "Merged file lost jane/combat's primary_ability_damage edit:\n{}",
            merged_content
        );
        assert!(
            merged_content.contains("@export var hazard_name: String = \"Molten Obsidian Geyser\""),
            "Merged file lost jane/hazards's hazard_name edit:\n{}",
            merged_content
        );
        assert!(
            merged_content.contains("@export var hazard_radius: float = 135.0"),
            "Merged file lost jane/hazards's hazard_radius edit:\n{}",
            merged_content
        );
    }

    #[tokio::test]
    async fn test_overlapping_line_edits_trigger_conflict_for_parent_resolution() {
        let repo_dir = make_temp_repo("overlapping_conflict");
        let base_script = concat!(
            "extends Node3D\n",
            "@export var boss_max_hp: int = 2400\n",
            "@export var arena_name: String = \"Boiling Iron Rig\"\n"
        );

        let script_rel = "scripts/zones/fire_prison_controller.gd";
        let script_full = repo_dir.join(script_rel);
        std::fs::create_dir_all(script_full.parent().unwrap()).unwrap();
        std::fs::write(&script_full, base_script).unwrap();

        let coordinator = DagGitCoordinator::new_in_repo(&repo_dir).unwrap();
        let base_sha = coordinator
            .start_stage1_pm_scoping(
                "jenny",
                "feat/conflict-detection-test",
                "Test conflicting edits on the same line",
            )
            .await
            .unwrap();

        // Sub-agent A sets `boss_max_hp` on line 2 to 3500
        let pr_a = coordinator
            .submit_subagent_branch_pr(
                "jane/alpha",
                "hp-3500",
                "Set boss_max_hp to 3500",
                vec![FileEdit::PatchReplace {
                    path: script_rel.to_string(),
                    old_text: "@export var boss_max_hp: int = 2400".to_string(),
                    new_text: "@export var boss_max_hp: int = 3500".to_string(),
                }],
                Some(&base_sha),
            )
            .await
            .unwrap();

        // Sub-agent B concurrently modifies the EXACT same line (`boss_max_hp`) to 9999 from `base_sha`
        let pr_b = coordinator
            .submit_subagent_branch_pr(
                "jane/beta",
                "hp-9999",
                "Set boss_max_hp to 9999",
                vec![FileEdit::PatchReplace {
                    path: script_rel.to_string(),
                    old_text: "@export var boss_max_hp: int = 2400".to_string(),
                    new_text: "@export var boss_max_hp: int = 9999".to_string(),
                }],
                Some(&base_sha),
            )
            .await
            .unwrap();

        // Parent agent `jenny` merges PR A cleanly
        let merge_a = coordinator
            .review_and_merge_pr("jenny", &pr_a.pr_id)
            .await
            .unwrap();
        assert_eq!(merge_a.status, MergeStatus::Merged);

        // Parent agent `jenny` attempts to merge PR B -> detects overlapping line conflict!
        let merge_b = coordinator
            .review_and_merge_pr("jenny", &pr_b.pr_id)
            .await
            .unwrap();
        assert_eq!(merge_b.status, MergeStatus::Conflict);
        assert!(merge_b
            .conflict_files
            .contains(&script_rel.to_string()));
        let details = merge_b.conflict_details.unwrap();
        assert!(
            details.contains("<<<<<<<") && details.contains(">>>>>>>"),
            "Expected standard Git conflict markers in conflict_details, got:\n{}",
            details
        );

        // Feature branch stays clean with PR A's value (3500) until parent resolves the conflict
        let current_feature_file = coordinator
            .read_file_on_feature_branch(script_rel)
            .await
            .unwrap();
        assert!(current_feature_file.contains("@export var boss_max_hp: int = 3500"));
        assert!(!current_feature_file.contains("<<<<<<<"));

        // Parent agent `jenny` resolves the conflict by choosing a balanced 5000 HP value
        let mut resolved_map = HashMap::new();
        resolved_map.insert(
            script_rel.to_string(),
            concat!(
                "extends Node3D\n",
                "@export var boss_max_hp: int = 5000\n",
                "@export var arena_name: String = \"Boiling Iron Rig\"\n"
            )
            .to_string(),
        );

        let resolved_res = coordinator
            .resolve_conflict_by_parent("jenny", &pr_b.pr_id, resolved_map)
            .await
            .unwrap();
        assert_eq!(resolved_res.status, MergeStatus::Merged);

        let final_file = coordinator
            .read_file_on_feature_branch(script_rel)
            .await
            .unwrap();
        assert!(final_file.contains("@export var boss_max_hp: int = 5000"));
    }
}
