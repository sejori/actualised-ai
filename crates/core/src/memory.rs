use std::path::{Path, PathBuf};

pub struct MemoryManager {
    base_dir: PathBuf,
}

impl MemoryManager {
    pub fn new(base_dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        #[cfg(not(target_arch = "wasm32"))]
        {
            std::fs::create_dir_all(&base_dir)?;
        }
        Ok(Self { base_dir })
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

        #[cfg(target_arch = "wasm32")]
        {
            // In a real WASM implementation, we would write to IndexedDB here.
            // For now, we mock the memory write to avoid file system panics.
            println!("[WASM Memory] Written to {}/memories/{}: {}", agent_id, file_name, content.len());
        }

        Ok(())
    }
}
