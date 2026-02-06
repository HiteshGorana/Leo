//! Skills module - extend agent capabilities

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Skill metadata from SKILL.md frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub requires: Vec<String>,
}

/// A loaded skill
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub requires: Vec<String>,
    pub content: String,
    pub path: PathBuf,
}

/// Skill registry - manages available skills
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    /// Create a new skill registry and load skills from workspace
    pub fn new(workspace: &Path) -> Self {
        let mut registry = Self {
            skills: HashMap::new(),
        };
        registry.load_from_directory(&workspace.join("skills"));
        registry
    }
    
    /// Create an empty registry for testing
    pub fn empty() -> Self {
        Self { skills: HashMap::new() }
    }
    
    /// Load skills from a directory
    fn load_from_directory(&mut self, path: &Path) {
        if !path.exists() {
            return;
        }
        
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        
        for entry in entries.filter_map(|e| e.ok()) {
            let skill_path = entry.path();
            if skill_path.is_dir() {
                self.load_skill(&skill_path);
            }
        }
    }
    
    /// Load a single skill from its directory
    fn load_skill(&mut self, path: &Path) {
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            return;
        }
        
        let Ok(content) = std::fs::read_to_string(&skill_md) else {
            return;
        };
        
        // Parse YAML frontmatter
        if let Some(skill) = parse_skill(&content, path) {
            self.skills.insert(skill.name.clone(), skill);
        }
    }
    
    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }
    
    /// List all skill names
    pub fn list(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }
    
    /// Check if a skill's requirements are met
    pub fn is_available(&self, name: &str, available_tools: &[&str]) -> bool {
        if let Some(skill) = self.skills.get(name) {
            skill.requires.iter().all(|r| available_tools.contains(&r.as_str()))
        } else {
            false
        }
    }
    
    /// Build skills summary for system prompt
    pub fn build_summary(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }
        
        let mut lines = vec!["<skills>".to_string()];
        
        for skill in self.skills.values() {
            lines.push(format!("  <skill name=\"{}\">", skill.name));
            lines.push(format!("    <description>{}</description>", skill.description));
            if !skill.requires.is_empty() {
                lines.push(format!("    <requires>{}</requires>", skill.requires.join(", ")));
            }
            lines.push("  </skill>".to_string());
        }
        
        lines.push("</skills>".to_string());
        lines.join("\n")
    }
}

/// Parse skill from SKILL.md content
fn parse_skill(content: &str, path: &Path) -> Option<Skill> {
    // Check for YAML frontmatter
    if !content.starts_with("---") {
        return None;
    }
    
    // Find end of frontmatter
    let rest = &content[3..];
    let end_idx = rest.find("\n---")?;
    let frontmatter = &rest[..end_idx];
    let body = &rest[end_idx + 5..];
    
    // Parse YAML (simplified - just key: value pairs)
    let mut name = None;
    let mut description = None;
    let mut requires = Vec::new();
    
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "name" => name = Some(value.to_string()),
                "description" => description = Some(value.to_string()),
                "requires" => {
                    // Parse array-like: [a, b, c] or just comma-separated
                    let value = value.trim_matches(['[', ']', ' '].as_ref());
                    requires = value.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
    }
    
    Some(Skill {
        name: name?,
        description: description.unwrap_or_default(),
        requires,
        content: body.trim().to_string(),
        path: path.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_skill() {
        let content = r#"---
name: weather
description: Get weather information
requires: [web_fetch]
---

# Weather Skill

Use this skill to get weather information.
"#;
        
        let skill = parse_skill(content, &PathBuf::from("/test")).unwrap();
        assert_eq!(skill.name, "weather");
        assert_eq!(skill.description, "Get weather information");
        assert_eq!(skill.requires, vec!["web_fetch"]);
        assert!(skill.content.contains("Weather Skill"));
    }
    
    #[test]
    fn test_skill_summary() {
        let mut registry = SkillRegistry::empty();
        registry.skills.insert("test".to_string(), Skill {
            name: "test".to_string(),
            description: "A test skill".to_string(),
            requires: vec![],
            content: String::new(),
            path: PathBuf::new(),
        });
        
        let summary = registry.build_summary();
        assert!(summary.contains("<skills>"));
        assert!(summary.contains("test"));
        assert!(summary.contains("A test skill"));
    }
}
