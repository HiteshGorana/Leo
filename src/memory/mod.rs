//! Memory module - persistent storage for agent state

use crate::Result;
use crate::error::Error;
use std::path::PathBuf;
use chrono::Local;

/// Memory store trait - interface for persistent memory
pub trait MemoryStore: Send + Sync {
    /// Get memory context for system prompt
    fn get_context(&self) -> Result<String>;
    
    /// Get long-term memory content
    fn read_long_term(&self) -> Result<String>;
    
    /// Append to long-term memory
    fn append_long_term(&self, content: &str) -> Result<()>;
    
    /// Get today's notes
    fn read_today(&self) -> Result<String>;
    
    /// Append to today's notes
    fn append_today(&self, content: &str) -> Result<()>;
}

/// File-based memory store
pub struct FileMemoryStore {
    workspace: PathBuf,
}

impl FileMemoryStore {
    pub fn new(workspace: &PathBuf) -> Self {
        Self {
            workspace: workspace.clone(),
        }
    }
    
    fn memory_dir(&self) -> PathBuf {
        self.workspace.join("memory")
    }
    
    fn long_term_path(&self) -> PathBuf {
        self.memory_dir().join("MEMORY.md")
    }
    
    fn today_path(&self) -> PathBuf {
        let today = Local::now().format("%Y-%m-%d").to_string();
        self.memory_dir().join("daily").join(format!("{}.md", today))
    }
}

impl MemoryStore for FileMemoryStore {
    fn get_context(&self) -> Result<String> {
        let mut parts = Vec::new();
        
        if let Ok(long_term) = self.read_long_term() {
            if !long_term.is_empty() {
                parts.push(format!("## Long-term Memory\n\n{}", long_term));
            }
        }
        
        if let Ok(today) = self.read_today() {
            if !today.is_empty() {
                parts.push(format!("## Today's Notes\n\n{}", today));
            }
        }
        
        Ok(parts.join("\n\n"))
    }
    
    fn read_long_term(&self) -> Result<String> {
        let path = self.long_term_path();
        if path.exists() {
            std::fs::read_to_string(&path).map_err(Error::from)
        } else {
            Ok(String::new())
        }
    }
    
    fn append_long_term(&self, content: &str) -> Result<()> {
        let path = self.long_term_path();
        
        // Create directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let mut current = if path.exists() {
            std::fs::read_to_string(&path)?
        } else {
            String::new()
        };
        
        current.push_str("\n");
        current.push_str(content);
        std::fs::write(&path, current)?;
        
        Ok(())
    }
    
    fn read_today(&self) -> Result<String> {
        let path = self.today_path();
        if path.exists() {
            std::fs::read_to_string(&path).map_err(Error::from)
        } else {
            Ok(String::new())
        }
    }
    
    fn append_today(&self, content: &str) -> Result<()> {
        let path = self.today_path();
        
        // Create directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let mut current = if path.exists() {
            std::fs::read_to_string(&path)?
        } else {
            format!("# Notes for {}\n\n", Local::now().format("%Y-%m-%d"))
        };
        
        current.push_str("\n");
        current.push_str(content);
        std::fs::write(&path, current)?;
        
        Ok(())
    }
}

/// In-memory store for testing
pub struct InMemoryStore {
    long_term: std::sync::Mutex<String>,
    today: std::sync::Mutex<String>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            long_term: std::sync::Mutex::new(String::new()),
            today: std::sync::Mutex::new(String::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStore for InMemoryStore {
    fn get_context(&self) -> Result<String> {
        let long_term = self.long_term.lock().unwrap();
        let today = self.today.lock().unwrap();
        
        let mut parts = Vec::new();
        if !long_term.is_empty() {
            parts.push(format!("## Long-term Memory\n\n{}", &*long_term));
        }
        if !today.is_empty() {
            parts.push(format!("## Today's Notes\n\n{}", &*today));
        }
        
        Ok(parts.join("\n\n"))
    }
    
    fn read_long_term(&self) -> Result<String> {
        Ok(self.long_term.lock().unwrap().clone())
    }
    
    fn append_long_term(&self, content: &str) -> Result<()> {
        let mut lt = self.long_term.lock().unwrap();
        lt.push_str("\n");
        lt.push_str(content);
        Ok(())
    }
    
    fn read_today(&self) -> Result<String> {
        Ok(self.today.lock().unwrap().clone())
    }
    
    fn append_today(&self, content: &str) -> Result<()> {
        let mut t = self.today.lock().unwrap();
        t.push_str("\n");
        t.push_str(content);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_in_memory_store() {
        let store = InMemoryStore::new();
        
        store.append_long_term("User likes coffee").unwrap();
        let lt = store.read_long_term().unwrap();
        assert!(lt.contains("coffee"));
        
        store.append_today("Discussed project plans").unwrap();
        let context = store.get_context().unwrap();
        assert!(context.contains("coffee"));
        assert!(context.contains("project plans"));
    }
}
