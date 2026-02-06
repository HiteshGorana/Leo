//! Bootstrap templates for workspace initialization.
//!
//! Templates are embedded at compile time from the `templates/` directory.
//! This ensures they're always available and versioned with the codebase.

/// Agent instructions - loaded every session
pub const AGENTS: &str = include_str!("../../templates/AGENTS.md");

/// Leo's identity and personality
pub const SOUL: &str = include_str!("../../templates/SOUL.md");

/// User information template
pub const USER: &str = include_str!("../../templates/USER.md");

/// Long-term memory template
pub const MEMORY: &str = include_str!("../../templates/MEMORY.md");

/// All template file names and their content
pub const TEMPLATES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS),
    ("SOUL.md", SOUL),
    ("USER.md", USER),
    ("MEMORY.md", MEMORY),
];

/// Create all bootstrap files in the given workspace directory.
/// Only creates files that don't already exist.
pub fn bootstrap_workspace(workspace: &std::path::Path) -> std::io::Result<()> {
    for (filename, content) in TEMPLATES {
        let path = workspace.join(filename);
        if !path.exists() {
            std::fs::write(&path, content)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_templates_not_empty() {
        assert!(!AGENTS.is_empty());
        assert!(!SOUL.is_empty());
        assert!(!USER.is_empty());
        assert!(!MEMORY.is_empty());
    }

    #[test]
    fn test_templates_have_headers() {
        assert!(AGENTS.contains("# AGENTS.md"));
        assert!(SOUL.contains("# SOUL.md"));
        assert!(USER.contains("# USER.md"));
        assert!(MEMORY.contains("# MEMORY.md"));
    }
}
