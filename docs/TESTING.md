# Testing Strategy

## Overview

Leo uses a layered testing approach:

1. **Unit Tests**: Test individual components in isolation
2. **Integration Tests**: Test component interactions
3. **Manual Tests**: Verify end-to-end behavior with real services

## Test Infrastructure

### FakeLlmClient

For testing agent logic without calling real LLMs:

```rust
// src/testing/fake_llm.rs

use crate::agent::llm::*;

pub struct FakeLlmClient {
    responses: VecDeque<LlmResponse>,
}

impl FakeLlmClient {
    /// Create with predefined responses (returned in order)
    pub fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: responses.iter()
                .map(|s| LlmResponse {
                    content: Some(s.to_string()),
                    tool_calls: vec![],
                    finish_reason: "stop".to_string(),
                    usage: Default::default(),
                })
                .collect(),
        }
    }

    /// Create with tool call response
    pub fn with_tool_call(name: &str, args: Value) -> Self {
        Self {
            responses: vec![
                LlmResponse {
                    content: None,
                    tool_calls: vec![ToolCallRequest {
                        id: "tc_1".to_string(),
                        name: name.to_string(),
                        arguments: args,
                    }],
                    finish_reason: "tool_calls".to_string(),
                    usage: Default::default(),
                },
                LlmResponse::text("Done!"),
            ].into(),
        }
    }
}

#[async_trait]
impl LlmClient for FakeLlmClient {
    async fn chat(&self, _: &[Message], _: &[ToolDefinition], _: &LlmConfig) -> Result<LlmResponse> {
        self.responses.pop_front()
            .ok_or_else(|| anyhow!("No more responses"))
    }

    fn default_model(&self) -> &str {
        "fake-model"
    }
}
```

### InMemoryStore

For testing without filesystem access:

```rust
// src/memory/in_memory.rs

use std::collections::HashMap;
use crate::memory::MemoryStore;

pub struct InMemoryStore {
    data: Mutex<HashMap<String, String>>,
}

impl MemoryStore for InMemoryStore {
    fn save(&self, key: &str, value: &str) -> Result<()> {
        self.data.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn load(&self, key: &str) -> Result<Option<String>> {
        Ok(self.data.lock().unwrap().get(key).cloned())
    }

    fn search(&self, query: &str) -> Result<Vec<MemoryEntry>> {
        // Simple substring search
        let data = self.data.lock().unwrap();
        Ok(data.iter()
            .filter(|(_, v)| v.contains(query))
            .map(|(k, v)| MemoryEntry { key: k.clone(), value: v.clone() })
            .collect())
    }

    fn get_context(&self) -> Result<String> {
        let data = self.data.lock().unwrap();
        Ok(data.values().cloned().collect::<Vec<_>>().join("\n"))
    }
}
```

### DummyToolRunner

For testing without executing real tools:

```rust
// src/testing/dummy_tools.rs

use crate::tools::{Tool, ToolRunner};

pub struct DummyTool {
    name: String,
    result: String,
}

#[async_trait]
impl Tool for DummyTool {
    fn name(&self) -> &str { &self.name }
    fn description(&self) -> &str { "dummy tool" }
    fn parameters(&self) -> Value { json!({"type": "object"}) }
    
    async fn execute(&self, _: Value) -> Result<String> {
        Ok(self.result.clone())
    }
}

impl ToolRunner {
    pub fn dummy() -> Self {
        let mut runner = Self::new();
        runner.register(DummyTool { 
            name: "read_file".to_string(), 
            result: "file contents".to_string() 
        });
        runner.register(DummyTool { 
            name: "web_search".to_string(), 
            result: "search results".to_string() 
        });
        runner
    }
}
```

---

## Unit Tests

### Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test tools::

# With output
cargo test -- --nocapture

# Single test
cargo test test_agent_loop_simple
```

### Example: Tool Validation

```rust
// tests/test_tools.rs

#[test]
fn test_validate_params_missing_required() {
    let tool = SampleTool::new();
    let errors = tool.validate(&json!({"query": "hi"}));
    assert!(errors.is_err());
    assert!(errors.unwrap_err().to_string().contains("count"));
}

#[test]
fn test_validate_params_type_and_range() {
    let tool = SampleTool::new();
    
    // Wrong type
    let errors = tool.validate(&json!({"query": "hi", "count": "2"}));
    assert!(errors.unwrap_err().to_string().contains("integer"));
    
    // Out of range
    let errors = tool.validate(&json!({"query": "hi", "count": 0}));
    assert!(errors.unwrap_err().to_string().contains("minimum"));
}
```

### Example: Agent Loop

```rust
// tests/test_agent_loop.rs

#[tokio::test]
async fn test_agent_loop_simple_conversation() {
    let client = FakeLlmClient::new(vec!["Hello, human!"]);
    let mut ctx = Context::test();
    let agent = AgentLoop::new(client, 10);
    
    let msg = InboundMessage {
        channel: "test".to_string(),
        sender_id: "user".to_string(),
        chat_id: "chat".to_string(),
        content: "Hi there".to_string(),
        timestamp: Utc::now(),
        media: vec![],
        metadata: Default::default(),
    };
    
    let response = agent.run(msg, &mut ctx).await.unwrap();
    assert_eq!(response.content, "Hello, human!");
}

#[tokio::test]
async fn test_agent_loop_with_tool_call() {
    let client = FakeLlmClient::with_tool_call("read_file", json!({"path": "test.txt"}));
    let mut ctx = Context::test();
    ctx.tool_runner = ToolRunner::dummy();
    let agent = AgentLoop::new(client, 10);
    
    let msg = InboundMessage::new("test", "Read test.txt");
    let response = agent.run(msg, &mut ctx).await.unwrap();
    
    assert_eq!(response.content, "Done!");
}

#[tokio::test]
async fn test_agent_loop_max_iterations() {
    // Create client that always returns tool calls
    let client = FakeLlmClient::always_tool_call("read_file");
    let mut ctx = Context::test();
    let agent = AgentLoop::new(client, 5);  // Max 5 iterations
    
    let msg = InboundMessage::new("test", "Read everything");
    let response = agent.run(msg, &mut ctx).await.unwrap();
    
    // Should hit max iterations
    assert!(response.content.contains("iterations"));
}
```

### Example: Memory Store

```rust
// tests/test_memory.rs

#[test]
fn test_in_memory_store_save_load() {
    let store = InMemoryStore::new();
    store.save("key1", "value1").unwrap();
    
    assert_eq!(store.load("key1").unwrap(), Some("value1".to_string()));
    assert_eq!(store.load("missing").unwrap(), None);
}

#[test]
fn test_in_memory_store_search() {
    let store = InMemoryStore::new();
    store.save("note1", "meeting with Alice").unwrap();
    store.save("note2", "meeting with Bob").unwrap();
    store.save("task", "buy groceries").unwrap();
    
    let results = store.search("meeting").unwrap();
    assert_eq!(results.len(), 2);
}
```

---

## Integration Tests

Located in `tests/` directory, test component interactions.

### Example: Full Agent Flow

```rust
// tests/integration/test_agent.rs

#[tokio::test]
async fn test_full_agent_flow() {
    // Setup
    let temp_dir = tempdir().unwrap();
    let config = Config::test(temp_dir.path());
    
    let client = FakeLlmClient::new(vec!["I'll check the file.", "The file contains: test content"]);
    let store = InMemoryStore::new();
    let skills = SkillRegistry::new(temp_dir.path());
    let tools = ToolRunner::new_with_defaults(&config);
    
    let mut ctx = Context {
        memory: Box::new(store),
        skills,
        tool_runner: tools,
        config: config.clone(),
    };
    
    let agent = AgentLoop::new(client, 10);
    
    // Write a test file
    std::fs::write(temp_dir.path().join("test.txt"), "test content").unwrap();
    
    // Run
    let msg = InboundMessage::new("test", "What's in test.txt?");
    let response = agent.run(msg, &mut ctx).await.unwrap();
    
    assert!(response.content.contains("test content"));
}
```

---

## Manual Testing

### CLI Commands

```bash
# After building
cargo build

# Test onboard
./target/debug/leo onboard

# Test agent
./target/debug/leo agent -m "What is 2+2?"

# Test status
./target/debug/leo status
```

### Telegram Bot

1. Create test bot via @BotFather
2. Configure token in `~/.leo/config.json`
3. Run: `./target/debug/leo gateway`
4. Send message to bot
5. Verify response

### Comparison with Python

Run same prompts through both implementations:

```bash
# Python
python -m leo agent -m "List files in current directory"

# Rust
./target/debug/leo agent -m "List files in current directory"

# Compare outputs
```

---

## Coverage

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Run with coverage
cargo tarpaulin --out Html

# View report
open tarpaulin-report.html
```

Target coverage: >80% for core modules.

---

## CI/CD

GitHub Actions workflow:

```yaml
# .github/workflows/rust.yml
name: Rust

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --all-features
      - run: cargo clippy -- -D warnings
      - run: cargo fmt -- --check
```

---

## Debugging Tests

```bash
# Enable logging
RUST_LOG=debug cargo test test_name -- --nocapture

# Run single test with backtrace
RUST_BACKTRACE=1 cargo test test_name

# Debug with lldb/gdb
cargo test test_name -- --test-threads=1
# Then attach debugger to test process
```
