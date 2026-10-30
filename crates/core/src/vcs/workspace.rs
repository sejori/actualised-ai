use super::VcsError;
use async_trait::async_trait;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use super::git_dag::{AgentWriteAuthorizer, DagGitCoordinator};

/// Canonically normalizes a path across game engines and operating systems.
///
/// Automatically:
/// - Trims leading and trailing whitespace.
/// - Strips Godot `res://` and `RES://` resource prefixes.
/// - Normalizes all backslashes (`\`) to forward slashes (`/`).
/// - Strips redundant leading `./` sequences.
/// - Strips leading slashes (`/`).
pub fn normalize_canonical_path(path: &str) -> String {
    let mut p = path.trim();
    if p.len() >= 6 && p[..6].eq_ignore_ascii_case("res://") {
        p = &p[6..];
    }
    let normalized = p.replace('\\', "/");
    let mut s = normalized.as_str();

    loop {
        if let Some(rest) = s.strip_prefix("./") {
            s = rest;
        } else if let Some(rest) = s.strip_prefix('/') {
            s = rest;
        } else {
            break;
        }
    }

    s.to_string()
}

/// Extensible workspace filesystem provider trait for autonomous AI agents.
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg(not(target_arch = "wasm32"))]
pub trait WorkspaceProvider: Send + Sync {
    /// Reads a file from the workspace on behalf of an agent.
    async fn read_file(&self, agent_id: &str, path: &str) -> Result<String, VcsError>;

    /// Writes content to a file in the workspace on behalf of an agent, optionally committing it.
    async fn write_file(
        &self,
        agent_id: &str,
        path: &str,
        content: &str,
        commit_message: Option<&str>,
    ) -> Result<(), VcsError>;

    /// Lists entries in a directory relative to the workspace root.
    async fn list_directory(&self, agent_id: &str, path: &str) -> Result<Vec<String>, VcsError>;
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg(target_arch = "wasm32")]
pub trait WorkspaceProvider {
    /// Reads a file from the workspace on behalf of an agent.
    async fn read_file(&self, agent_id: &str, path: &str) -> Result<String, VcsError>;

    /// Writes content to a file in the workspace on behalf of an agent, optionally committing it.
    async fn write_file(
        &self,
        agent_id: &str,
        path: &str,
        content: &str,
        commit_message: Option<&str>,
    ) -> Result<(), VcsError>;

    /// Lists entries in a directory relative to the workspace root.
    async fn list_directory(&self, agent_id: &str, path: &str) -> Result<Vec<String>, VcsError>;
}

/// Local Git-backed workspace provider wrapping `DagGitCoordinator`.
///
/// Features:
/// - Canonical path normalization (stripping `res://`, `./`, `/`, `\`).
/// - Strict write authorization and path traversal prevention via `AgentWriteAuthorizer`.
/// - Automatic refactor barrier synchronization (`wait_if_refactor_active`).
/// - Real Git staging, commits, and multi-branch/worktree fallback reads.
#[cfg(not(target_arch = "wasm32"))]
pub struct LocalGitWorkspaceProvider {
    coordinator: Arc<DagGitCoordinator>,
}

#[cfg(not(target_arch = "wasm32"))]
impl LocalGitWorkspaceProvider {
    /// Creates a provider wrapping an existing `DagGitCoordinator`.
    pub fn new(coordinator: Arc<DagGitCoordinator>) -> Self {
        Self { coordinator }
    }

    /// Initializes a local git repository at `repo_path` with `DefaultWriteAuthorizer`.
    pub fn new_in_repo(repo_path: impl Into<PathBuf>) -> Result<Self, VcsError> {
        let coord = DagGitCoordinator::new_in_repo(repo_path)
            .map_err(|e| VcsError(e))?;
        Ok(Self { coordinator: coord })
    }

    /// Initializes a local git repository at `repo_path` with a custom `AgentWriteAuthorizer`.
    pub fn new_with_authorizer(
        repo_path: impl Into<PathBuf>,
        authorizer: Arc<dyn AgentWriteAuthorizer>,
    ) -> Result<Self, VcsError> {
        let coord = DagGitCoordinator::new_in_repo_with_authorizer(repo_path, authorizer)
            .map_err(|e| VcsError(e))?;
        Ok(Self { coordinator: coord })
    }

    /// Creates a provider backed by a sandbox mirrored from `workspace_path`.
    pub fn new_with_sandbox_for_workspace(workspace_path: impl Into<PathBuf>) -> Result<Self, VcsError> {
        let coord = DagGitCoordinator::new_with_sandbox_for_workspace(workspace_path)
            .map_err(|e| VcsError(e))?;
        Ok(Self { coordinator: coord })
    }

    /// Creates a provider backed by a sandbox mirrored from `workspace_path` with a custom authorizer.
    pub fn new_with_sandbox_and_authorizer(
        workspace_path: impl Into<PathBuf>,
        authorizer: Arc<dyn AgentWriteAuthorizer>,
    ) -> Result<Self, VcsError> {
        let coord = DagGitCoordinator::new_with_sandbox_and_authorizer(workspace_path, authorizer)
            .map_err(|e| VcsError(e))?;
        Ok(Self { coordinator: coord })
    }

    /// Returns a reference to the underlying `DagGitCoordinator`.
    pub fn coordinator(&self) -> &Arc<DagGitCoordinator> {
        &self.coordinator
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl WorkspaceProvider for LocalGitWorkspaceProvider {
    async fn read_file(&self, _agent_id: &str, path: &str) -> Result<String, VcsError> {
        let clean = normalize_canonical_path(path);
        if clean.is_empty() || clean.contains("..") {
            return Err(VcsError("Invalid or traversal file path".to_string()));
        }

        // 1. Direct working tree disk read
        let full_path = self.coordinator.git_repo_path.join(&clean);
        if full_path.is_file() {
            return std::fs::read_to_string(&full_path)
                .map_err(|e| VcsError(format!("Failed to read {}: {}", clean, e)));
        }

        // 2. Workspace sync path fallback
        if let Some(ref ws_root) = self.coordinator.workspace_sync_path {
            let sync_file = ws_root.join(&clean);
            if sync_file.is_file() {
                return std::fs::read_to_string(&sync_file)
                    .map_err(|e| VcsError(format!("Failed to read {}: {}", clean, e)));
            }
        }

        // 3. Fallback to feature branch tip in Git
        if let Ok(content) = self.coordinator.read_file_on_feature_branch(&clean).await {
            return Ok(content);
        }

        // 4. Fallback to Stage 3 base commit in Git
        if let Ok(content) = self.coordinator.read_file_at_base(&clean).await {
            return Ok(content);
        }

        Err(VcsError(format!("File '{}' not found in workspace", clean)))
    }

    async fn write_file(
        &self,
        agent_id: &str,
        path: &str,
        content: &str,
        commit_message: Option<&str>,
    ) -> Result<(), VcsError> {
        let clean = normalize_canonical_path(path);
        if clean.is_empty() || clean.contains("..") {
            return Err(VcsError("Invalid or traversal file path".to_string()));
        }

        // 1. Check write permission with authorizer
        self.coordinator
            .authorizer
            .check_write_permission(agent_id, &clean)
            .map_err(VcsError)?;

        // 2. Await refactor barrier if Stage 2 lock is active
        self.coordinator.wait_if_refactor_active(agent_id).await;

        // 3. Write file in Git working tree
        let full_path = self.coordinator.git_repo_path.join(&clean);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| VcsError(format!("Failed to create parent directory for {}: {}", clean, e)))?;
        }
        std::fs::write(&full_path, content)
            .map_err(|e| VcsError(format!("Failed to write {}: {}", clean, e)))?;

        // 4. Mirror to workspace_sync_path if configured
        if let Some(ref ws_root) = self.coordinator.workspace_sync_path {
            let dst = ws_root.join(&clean);
            if let Some(parent) = dst.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&dst, content);
        }

        // 5. Commit to local git repository
        let default_msg = format!("chore({}): update {}", agent_id, clean);
        let commit_msg = commit_message.unwrap_or(&default_msg);

        self.coordinator
            .commit_file_on_working_tree(&clean, commit_msg)
            .await
            .map_err(VcsError)?;

        Ok(())
    }

    async fn list_directory(&self, _agent_id: &str, path: &str) -> Result<Vec<String>, VcsError> {
        let clean = normalize_canonical_path(path);
        if clean.contains("..") {
            return Err(VcsError("Path traversal ('..') is strictly prohibited".to_string()));
        }

        let dir = if clean.is_empty() || clean == "." {
            self.coordinator.git_repo_path.clone()
        } else {
            self.coordinator.git_repo_path.join(&clean)
        };

        if dir.is_dir() {
            let mut entries = Vec::new();
            let rd = std::fs::read_dir(&dir)
                .map_err(|e| VcsError(format!("Failed to read directory '{}': {}", clean, e)))?;
            for entry in rd {
                let entry = entry.map_err(|e| VcsError(e.to_string()))?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name == ".git" {
                    continue;
                }
                entries.push(name);
            }
            entries.sort();
            return Ok(entries);
        }

        // Fallback: check workspace_sync_path
        if let Some(ref ws_root) = self.coordinator.workspace_sync_path {
            let sync_dir = if clean.is_empty() || clean == "." {
                ws_root.clone()
            } else {
                ws_root.join(&clean)
            };
            if sync_dir.is_dir() {
                let mut entries = Vec::new();
                let rd = std::fs::read_dir(&sync_dir)
                    .map_err(|e| VcsError(format!("Failed to read directory '{}': {}", clean, e)))?;
                for entry in rd {
                    let entry = entry.map_err(|e| VcsError(e.to_string()))?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name == ".git" {
                        continue;
                    }
                    entries.push(name);
                }
                entries.sort();
                return Ok(entries);
            }
        }

        Err(VcsError(format!("Directory '{}' not found in workspace", clean)))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl super::VersionControl for LocalGitWorkspaceProvider {
    async fn read_file(&self, path: &str) -> Result<String, VcsError> {
        WorkspaceProvider::read_file(self, "system", path).await
    }

    async fn write_file(&self, path: &str, content: &str, message: &str) -> Result<(), VcsError> {
        WorkspaceProvider::write_file(self, "system", path, content, Some(message)).await
    }

    async fn list_directory(&self, path: &str) -> Result<Vec<String>, VcsError> {
        WorkspaceProvider::list_directory(self, "system", path).await
    }

    async fn search_codebase(&self, query: &str) -> Result<Vec<String>, VcsError> {
        let q = query.trim();
        if q.is_empty() {
            return Ok(Vec::new());
        }
        let out = DagGitCoordinator::run_git_raw(
            &self.coordinator.git_repo_path,
            &["grep", "-l", "--ignore-case", q],
        );
        match out {
            Ok(stdout) => Ok(stdout
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()),
            Err(_) => Ok(Vec::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_path_normalization() {
        assert_eq!(
            normalize_canonical_path("res://scenes/zones/fire_prison.tscn"),
            "scenes/zones/fire_prison.tscn"
        );
        assert_eq!(
            normalize_canonical_path("RES://scenes/zones/fire_prison.tscn"),
            "scenes/zones/fire_prison.tscn"
        );
        assert_eq!(
            normalize_canonical_path("res:///specs/gdd_design.json"),
            "specs/gdd_design.json"
        );
        assert_eq!(
            normalize_canonical_path("./specs/gdd_design.json"),
            "specs/gdd_design.json"
        );
        assert_eq!(
            normalize_canonical_path("././dialogues/intro.dialogue"),
            "dialogues/intro.dialogue"
        );
        assert_eq!(
            normalize_canonical_path("/specs/interface_contracts.json"),
            "specs/interface_contracts.json"
        );
        assert_eq!(
            normalize_canonical_path(r"\specs\interface_contracts.json"),
            "specs/interface_contracts.json"
        );
        assert_eq!(
            normalize_canonical_path(r".\scripts\player.gd"),
            "scripts/player.gd"
        );
        assert_eq!(
            normalize_canonical_path(r"res://\scenes\player.tscn"),
            "scenes/player.tscn"
        );
        assert_eq!(
            normalize_canonical_path("   res://specs/gdd.json   "),
            "specs/gdd.json"
        );
        assert_eq!(
            normalize_canonical_path("scripts/player.gd"),
            "scripts/player.gd"
        );
        assert_eq!(normalize_canonical_path(""), "");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_local_git_workspace_read_write_commit_and_normalize() {
        let temp_dir = std::env::temp_dir().join(format!("actualised_ws_test_{}", uuid::Uuid::new_v4().simple()));
        let provider = LocalGitWorkspaceProvider::new_in_repo(&temp_dir).unwrap();

        // Write using Godot res:// prefix
        provider
            .write_file("jenny", "res://specs/gdd_design.json", "{\"title\": \"Fire Realm\"}", Some("feat: add gdd"))
            .await
            .unwrap();

        // Read back with various normalized prefixes
        let content1 = provider.read_file("john", "specs/gdd_design.json").await.unwrap();
        assert_eq!(content1, "{\"title\": \"Fire Realm\"}");

        let content2 = provider.read_file("john", "./specs/gdd_design.json").await.unwrap();
        assert_eq!(content2, "{\"title\": \"Fire Realm\"}");

        let content3 = provider.read_file("john", "/specs/gdd_design.json").await.unwrap();
        assert_eq!(content3, "{\"title\": \"Fire Realm\"}");

        let content4 = provider.read_file("john", "res://specs/gdd_design.json").await.unwrap();
        assert_eq!(content4, "{\"title\": \"Fire Realm\"}");

        // List directory
        let files = provider.list_directory("geoff", "res://specs").await.unwrap();
        assert_eq!(files, vec!["gdd_design.json".to_string()]);

        // Cleanup
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_local_git_workspace_traversal_rejection() {
        let temp_dir = std::env::temp_dir().join(format!("actualised_ws_test_{}", uuid::Uuid::new_v4().simple()));
        let provider = LocalGitWorkspaceProvider::new_in_repo(&temp_dir).unwrap();

        let err1 = provider.read_file("agent", "../secret.txt").await.unwrap_err();
        assert!(err1.0.contains("traversal"));

        let err2 = provider.write_file("agent", "res://../escape.txt", "content", None).await.unwrap_err();
        assert!(err2.0.contains("traversal") || err2.0.contains("prohibited"));

        let err3 = provider.list_directory("agent", "specs/../../").await.unwrap_err();
        assert!(err3.0.contains("traversal"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_local_git_workspace_custom_authorizer() {
        use super::AgentWriteAuthorizer;

        struct StrictAuthorizer;
        impl AgentWriteAuthorizer for StrictAuthorizer {
            fn is_parent_agent(&self, agent_id: &str) -> bool {
                agent_id == "lead"
            }
            fn check_write_permission(&self, agent_id: &str, rel_path: &str) -> Result<(), String> {
                if rel_path.starts_with("system/") && agent_id != "lead" {
                    return Err("Only lead agent may write to system/".to_string());
                }
                Ok(())
            }
        }

        let temp_dir = std::env::temp_dir().join(format!("actualised_ws_test_{}", uuid::Uuid::new_v4().simple()));
        let provider = LocalGitWorkspaceProvider::new_with_authorizer(&temp_dir, Arc::new(StrictAuthorizer)).unwrap();

        // Subagent writing to system/ is rejected
        let res_sub = provider.write_file("subagent_1", "res://system/core.json", "{}", None).await;
        assert!(res_sub.is_err());
        assert!(res_sub.unwrap_err().0.contains("Only lead agent"));

        // Lead writing to system/ succeeds
        let res_lead = provider.write_file("lead", "res://system/core.json", "{}", None).await;
        assert!(res_lead.is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
