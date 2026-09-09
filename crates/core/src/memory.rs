use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryFile {
    pub file_name: String,
    pub content: String,
}

pub struct MemoryManager {
    base_dir: PathBuf,
    // agent_id -> (file_name -> content); kept in-memory so it's readable from the UI on every target,
    // including wasm where there's no real filesystem.
    memories: Mutex<HashMap<String, HashMap<String, String>>>,
}

impl MemoryManager {
    pub fn new(base_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::create_dir_all(&base_dir)?;
        }
        Ok(Self { base_dir, memories: Mutex::new(HashMap::new()) })
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

    /// Returns every memory file written by an agent so far.
    pub fn read_memories(&self, agent_id: &str) -> Vec<MemoryFile> {
        self.memories
            .lock()
            .unwrap()
            .get(agent_id)
            .map(|files| files.iter().map(|(file_name, content)| MemoryFile { file_name: file_name.clone(), content: content.clone() }).collect())
            .unwrap_or_default()
    }
}
