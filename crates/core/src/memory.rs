use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryFile {
    pub file_name: String,
    pub content: String,
}

/// A folder/file tree, built on demand from the flat path -> content maps so the UI
/// can render it with nested `<details>`/`<summary>` elements.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind")]
pub enum MemoryNode {
    #[serde(rename = "folder")]
    Folder { name: String, children: Vec<MemoryNode> },
    #[serde(rename = "file")]
    File { name: String, path: String, content: String },
}

/// Builds a nested tree from flat `"a/b/c.md"` style paths, folders first then files, both alphabetical.
fn build_tree(files: &HashMap<String, String>) -> Vec<MemoryNode> {
    let mut entries: Vec<(&str, &str)> = files.iter().map(|(p, c)| (p.as_str(), c.as_str())).collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    build_level(&entries, "")
}

fn build_level(entries: &[(&str, &str)], prefix: &str) -> Vec<MemoryNode> {
    let mut folder_order: Vec<String> = Vec::new();
    let mut folder_entries: HashMap<String, Vec<(&str, &str)>> = HashMap::new();
    let mut files_here: Vec<(&str, &str)> = Vec::new();

    for &(path, content) in entries {
        let rel = path.strip_prefix(prefix).unwrap_or(path).trim_start_matches('/');
        if rel.is_empty() {
            continue;
        }
        match rel.split_once('/') {
            Some((folder, _rest)) => {
                folder_entries.entry(folder.to_string()).or_insert_with(|| {
                    folder_order.push(folder.to_string());
                    Vec::new()
                }).push((path, content));
            }
            None => files_here.push((path, content)),
        }
    }

    let mut nodes = Vec::new();
    folder_order.sort();
    for folder in folder_order {
        let child_entries = folder_entries.remove(&folder).unwrap_or_default();
        let next_prefix = if prefix.is_empty() { folder.clone() } else { format!("{}/{}", prefix, folder) };
        nodes.push(MemoryNode::Folder { children: build_level(&child_entries, &next_prefix), name: folder });
    }

    files_here.sort_by(|a, b| a.0.cmp(b.0));
    for (path, content) in files_here {
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        nodes.push(MemoryNode::File { name, path: path.to_string(), content: content.to_string() });
    }

    nodes
}

pub struct MemoryManager {
    base_dir: PathBuf,
    // agent_id -> (path -> content); kept in-memory so it's readable from the UI on every target,
    // including wasm where there's no real filesystem. Paths may contain '/' to express folders.
    memories: Mutex<HashMap<String, HashMap<String, String>>>,
    // A single tree shared by every agent/team, for cross-team artefacts (specs, shared docs, etc.).
    shared: Mutex<HashMap<String, String>>,
}

impl MemoryManager {
    pub fn new(base_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::create_dir_all(&base_dir)?;
        }
        Ok(Self { base_dir, memories: Mutex::new(HashMap::new()), shared: Mutex::new(HashMap::new()) })
    }

    pub fn setup_agent_dir(&self, agent_id: &str) -> std::io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let agent_dir = self.base_dir.join(agent_id);
            let memories_dir = agent_dir.join("memories");
            std::fs::create_dir_all(&memories_dir)?;
            
            let index_path = agent_dir.join("index.md");
            if !index_path.exists() {
                std::fs::write(index_path, format!("# Index for Agent {}\n", agent_id))?;
            }
        }
        
        Ok(())
    }

    pub fn write_memory(&self, agent_id: &str, file_name: &str, content: &str) -> std::io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let memories_dir = self.base_dir.join(agent_id).join("memories");
            if let Some(parent) = Path::new(file_name).parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(memories_dir.join(parent))?;
            }
            std::fs::write(memories_dir.join(file_name), content)?;
        }

        self.memories
            .lock()
            .unwrap()
            .entry(agent_id.to_string())
            .or_default()
            .insert(file_name.to_string(), content.to_string());

        Ok(())
    }

    /// Returns every memory file written by an agent so far, as a flat list.
    pub fn read_memories(&self, agent_id: &str) -> Vec<MemoryFile> {
        self.memories
            .lock()
            .unwrap()
            .get(agent_id)
            .map(|files| files.iter().map(|(file_name, content)| MemoryFile { file_name: file_name.clone(), content: content.clone() }).collect())
            .unwrap_or_default()
    }

    /// Returns an agent's memories organised as a nested folder/file tree.
    pub fn read_memory_tree(&self, agent_id: &str) -> Vec<MemoryNode> {
        let memories = self.memories.lock().unwrap();
        match memories.get(agent_id) {
            Some(files) => build_tree(files),
            None => Vec::new(),
        }
    }

    /// Writes to the shared team directory, visible to every agent.
    pub fn write_shared(&self, path: &str, content: &str) -> std::io::Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let shared_dir = self.base_dir.join("_shared");
            if let Some(parent) = Path::new(path).parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(shared_dir.join(parent))?;
            } else {
                std::fs::create_dir_all(&shared_dir)?;
            }
            std::fs::write(shared_dir.join(path), content)?;
        }

        self.shared.lock().unwrap().insert(path.to_string(), content.to_string());
        Ok(())
    }

    /// Returns the shared team directory as a nested folder/file tree.
    pub fn read_shared_tree(&self) -> Vec<MemoryNode> {
        build_tree(&self.shared.lock().unwrap())
    }
}

