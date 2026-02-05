//! Context builder for agent prompts

use std::path::PathBuf;
use crate::Result;
use crate::config::Config;
use crate::memory::MemoryStore;
use crate::skills::SkillRegistry;
use crate::tools::ToolRunner;
use super::message::Message;

/// Context holds all state for an agent interaction
pub struct Context {
    pub memory: Box<dyn MemoryStore>,
    pub skills: SkillRegistry,
    pub tool_runner: ToolRunner,
    pub workspace: PathBuf,
    pub config: Config,
}

impl Context {
    /// Create a new context from configuration
    pub fn new(config: &Config) -> Result<Self> {
        use crate::memory::FileMemoryStore;
        
        let memory = Box::new(FileMemoryStore::new(&config.workspace));
        let skills = SkillRegistry::new(&config.workspace);
        let tool_runner = ToolRunner::new_with_defaults(&config.workspace);
        
        Ok(Self {
            memory,
            skills,
            tool_runner,
            workspace: config.workspace.clone(),
            config: config.clone(),
        })
    }
    
    /// Create a test context with in-memory components
    #[cfg(test)]
    pub fn test() -> Self {
        use crate::memory::InMemoryStore;
        
        Self {
            memory: Box::new(InMemoryStore::new()),
            skills: SkillRegistry::empty(),
            tool_runner: ToolRunner::new(),
            workspace: PathBuf::from("/tmp/test"),
            config: Config::default(),
        }
    }
    
    /// Build system prompt from memory, skills, and bootstrap files
    pub fn build_system_prompt(&self) -> String {
        let mut parts = vec![self.get_identity()];
        
        // Load bootstrap files
        if let Ok(content) = self.load_bootstrap_files() {
            if !content.is_empty() {
                parts.push(content);
            }
        }
        
        // Memory context
        if let Ok(memory) = self.memory.get_context() {
            if !memory.is_empty() {
                parts.push(format!("# Memory\n\n{}", memory));
            }
        }
        
        // Skills summary
        let skills_summary = self.skills.build_summary();
        if !skills_summary.is_empty() {
            parts.push(format!(
                "# Skills\n\nThe following skills extend your capabilities:\n\n{}",
                skills_summary
            ));
        }
        
        parts.join("\n\n---\n\n")
    }
    
    /// Build messages list for LLM call
    pub fn build_messages(&self, history: &[Message], current: &str) -> Vec<Message> {
        let mut messages = Vec::new();
        
        // System prompt
        messages.push(Message::system(self.build_system_prompt()));
        
        // History
        messages.extend(history.iter().cloned());
        
        // Current message
        messages.push(Message::user(current));
        
        messages
    }
    
    fn get_identity(&self) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M (%A)");
        let workspace = self.workspace.display();
        
        format!(
            r#"# Leo 🦁

You are Leo, a helpful AI assistant. You have access to tools that allow you to:
- Read, write, and edit files
- Execute shell commands
- Search the web and fetch web pages

## Current Time
{}

## Workspace
Your workspace is at: {}

Always be helpful, accurate, and concise. When using tools, explain what you're doing."#,
            now, workspace
        )
    }
    
    fn load_bootstrap_files(&self) -> Result<String> {
        let bootstrap_files = ["AGENTS.md", "SOUL.md", "USER.md", "IDENTITY.md", "TOOLS.md"];
        let mut parts = Vec::new();
        
        for filename in bootstrap_files {
            let path = self.workspace.join(filename);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    parts.push(format!("## {}\n\n{}", filename, content));
                }
            }
        }
        
        Ok(parts.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_context_build_system_prompt() {
        let ctx = Context::test();
        let prompt = ctx.build_system_prompt();
        assert!(prompt.contains("Leo"));
        assert!(prompt.contains("helpful AI assistant"));
    }
    
    #[test]
    fn test_context_build_messages() {
        let ctx = Context::test();
        let messages = ctx.build_messages(&[], "Hello");
        
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, super::super::message::Role::System);
        assert_eq!(messages[1].role, super::super::message::Role::User);
        assert_eq!(messages[1].content, "Hello");
    }
}
