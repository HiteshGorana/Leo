//! Context builder for agent prompts.
//!
//! Optimizations implemented:
//! - Bootstrap files cached at construction time
//! - History windowing (max N messages)
//! - Pre-allocated string buffers

use std::path::PathBuf;

use crate::config::Config;
use crate::memory::MemoryStore;
use crate::skills::SkillRegistry;
use crate::tools::ToolRunner;
use crate::Result;

use super::message::Message;

/// Maximum history messages to include in prompt (prevents unbounded growth).
const MAX_HISTORY_MESSAGES: usize = 40;

/// Context holds all state for an agent interaction.
pub struct Context {
    pub memory: Box<dyn MemoryStore>,
    pub skills: SkillRegistry,
    pub tool_runner: ToolRunner,
    pub workspace: PathBuf,
    pub config: Config,
    /// Cached bootstrap file content (loaded once at construction).
    cached_bootstrap: String,
}

impl Context {
    /// Create a new context from configuration.
    pub fn new(config: &Config) -> Result<Self> {
        use crate::memory::FileMemoryStore;

        let memory = Box::new(FileMemoryStore::new(&config.workspace));
        let skills = SkillRegistry::new(&config.workspace);
        let tool_runner = ToolRunner::new_with_defaults(&config.workspace);

        // Cache bootstrap files at construction time
        let cached_bootstrap = Self::load_bootstrap_files_static(&config.workspace);

        Ok(Self {
            memory,
            skills,
            tool_runner,
            workspace: config.workspace.clone(),
            config: config.clone(),
            cached_bootstrap,
        })
    }

    /// Create a test context with in-memory components.
    #[cfg(test)]
    pub fn test() -> Self {
        use crate::memory::InMemoryStore;

        Self {
            memory: Box::new(InMemoryStore::new()),
            skills: SkillRegistry::empty(),
            tool_runner: ToolRunner::new(),
            workspace: PathBuf::from("/tmp/test"),
            config: Config::default(),
            cached_bootstrap: String::new(),
        }
    }

    /// Build system prompt from memory, skills, and cached bootstrap files.
    pub fn build_system_prompt(&self) -> String {
        // Estimate capacity to reduce allocations
        let mut parts = Vec::with_capacity(4);
        parts.push(self.get_identity());

        // Use cached bootstrap files (loaded at construction)
        if !self.cached_bootstrap.is_empty() {
            parts.push(self.cached_bootstrap.clone());
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

    /// Build messages list for LLM call with history windowing.
    pub fn build_messages(&self, history: &[Message], current: &str) -> Vec<Message> {
        // Apply history windowing to prevent unbounded growth
        let windowed_history = if history.len() > MAX_HISTORY_MESSAGES {
            &history[history.len() - MAX_HISTORY_MESSAGES..]
        } else {
            history
        };

        // Pre-allocate with estimated capacity
        let mut messages = Vec::with_capacity(windowed_history.len() + 2);

        // System prompt
        messages.push(Message::system(self.build_system_prompt()));

        // History (windowed)
        messages.extend(windowed_history.iter().cloned());

        // Current message
        messages.push(Message::user(current));

        messages
    }

    /// Reload bootstrap files (call if files changed during session).
    pub fn reload_bootstrap(&mut self) {
        self.cached_bootstrap = Self::load_bootstrap_files_static(&self.workspace);
    }

    fn get_identity(&self) -> String {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M (%A)");
        let workspace = self.workspace.display();

        format!(
            r#"# Leo 🦁

You are Leo, an intelligent AI Personal Assistant running locally. 🦁

## Current Time
{}

## Workspace
Your workspace is: `{}`

**IMPORTANT**: When the user mentions files or folders without full paths:
- Assume they mean files inside the workspace above
- Use relative paths from the workspace (e.g., "src/main.rs" not "/full/path/src/main.rs")
- NEVER ask for the full path - just try the relative path first
- Common locations: "documents" = ~/Documents, "desktop" = ~/Desktop

## Tools
You have access to these tools:
- `read_file`, `write_file`, `edit`, `list_dir` - File operations
- `find_files` - Find files by pattern (*.rs, config*, etc.)
- `search` - Search text in files (supports regex or literal)
- `exec` - Run shell commands
- `git` - Git operations
- `web_search`, `web_fetch` - Web access
- `memory` - Long-term memory (read/add)

## Memory Instructions
**CRITICAL**: When the user tells you to remember ANYTHING - names, preferences, identity, aim, purpose:

Just use the memory tool immediately:
```
memory(action="add", content="[what to remember]")
```

Examples:
- "Call me X" → memory(action="add", content="Owner's name is X")
- "Your name is Cat" → memory(action="add", content="My name is now Cat")
- "Your aim is to help me code" → memory(action="add", content="My aim is to help with coding")

Do NOT read multiple files. Just save to memory and confirm.

Always be helpful, accurate, and concise. When using tools, just do it—don't explain unless asked."#,
            now, workspace
        )
    }

    /// Load bootstrap files from workspace (static helper for caching).
    fn load_bootstrap_files_static(workspace: &std::path::Path) -> String {
        const BOOTSTRAP_FILES: [&str; 6] = [
            "AGENTS.md",
            "SOUL.md",
            "USER.md",
            "IDENTITY.md",
            "TOOLS.md",
            "MEMORY.md",
        ];

        let mut parts = Vec::with_capacity(BOOTSTRAP_FILES.len());

        for filename in BOOTSTRAP_FILES {
            let path = workspace.join(filename);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    parts.push(format!("## {}\n\n{}", filename, content));
                }
            }
        }

        parts.join("\n\n")
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

    #[test]
    fn test_history_windowing() {
        let ctx = Context::test();

        // Create large history
        let mut history = Vec::new();
        for i in 0..100 {
            history.push(Message::user(format!("Message {}", i)));
        }

        let messages = ctx.build_messages(&history, "Current");

        // Should have: system + MAX_HISTORY_MESSAGES + current
        assert_eq!(messages.len(), MAX_HISTORY_MESSAGES + 2);

        // Last history message should be the most recent
        let last_history_msg = &messages[messages.len() - 2];
        assert!(last_history_msg.content.contains("99"));
    }
}
