# Extending Leo

This guide explains how to extend Leo with custom skills, tools, and adapters.

## Adding a Skill

Skills are markdown files that teach the agent how to perform specific tasks.

### 1. Create the Skill Directory

```bash
mkdir -p ~/.leo/workspace/skills/my-skill
```

### 2. Create SKILL.md

```markdown
---
name: my-skill
description: A custom skill for [purpose]
---

# My Skill

## When to Use
Use this skill when the user asks about [topic].

## Instructions
1. First, do X
2. Then, do Y
3. Finally, do Z

## Examples
- Example input: "..."
- Expected action: "..."
```

### 3. Optional: Add Requirements

```markdown
---
name: my-skill
description: Skill requiring external tools
metadata: {"leo": {"requires": {"bins": ["ffmpeg"], "env": ["MY_API_KEY"]}}}
---
```

### 4. Always-Active Skills

For skills that should always be loaded:

```markdown
---
name: always-on
description: Critical skill
always: true
---
```

---

## Adding a Tool

Tools are Rust structs that implement the `Tool` trait.

### 1. Define the Tool

```rust
// src/tools/my_tool.rs

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::tools::Tool;
use crate::error::Result;

pub struct MyTool {
    // Tool state/config
}

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str {
        "my_tool"
    }

    fn description(&self) -> &str {
        "Does something useful"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "The input to process"
                },
                "count": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            },
            "required": ["input"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let input = params["input"].as_str().unwrap_or("");
        let count = params["count"].as_i64().unwrap_or(1);
        
        // Do something...
        
        Ok(format!("Processed {} x{}", input, count))
    }
}
```

### 2. Register the Tool

```rust
// src/tools/mod.rs

mod my_tool;
pub use my_tool::MyTool;

// In ToolRunner::new_with_defaults()
pub fn new_with_defaults(config: &Config) -> Self {
    let mut runner = Self::new();
    runner.register(ReadFileTool::new());
    runner.register(WriteFileTool::new());
    // ... other tools ...
    runner.register(MyTool::new());  // Add your tool
    runner
}
```

### 3. Add Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_tool_basic() {
        let tool = MyTool::new();
        let result = tool.execute(json!({
            "input": "hello",
            "count": 3
        })).await.unwrap();
        
        assert_eq!(result, "Processed hello x3");
    }

    #[tokio::test]
    async fn test_my_tool_validation() {
        let tool = MyTool::new();
        let err = tool.validate(&json!({})).unwrap_err();
        assert!(err.to_string().contains("required"));
    }
}
```

---

## Adding an Adapter (Channel)

Adapters connect Leo to chat platforms.

### 1. Implement the Channel Trait

```rust
// src/adapters/my_channel.rs

use async_trait::async_trait;
use crate::adapters::Channel;
use crate::agent::message::{InboundMessage, Response};
use crate::error::Result;

pub struct MyChannelConfig {
    pub enabled: bool,
    pub api_key: String,
    pub allow_from: Vec<String>,
}

pub struct MyChannel {
    config: MyChannelConfig,
    inbound_tx: mpsc::Sender<InboundMessage>,
}

#[async_trait]
impl Channel for MyChannel {
    fn name(&self) -> &str {
        "my_channel"
    }

    async fn start(&mut self) -> Result<()> {
        // Connect to the platform
        // Start receiving messages
        // Forward to inbound_tx
        
        loop {
            let msg = self.receive_from_platform().await?;
            
            if self.is_allowed(&msg.sender_id) {
                let inbound = InboundMessage {
                    channel: self.name().to_string(),
                    sender_id: msg.sender_id,
                    chat_id: msg.chat_id,
                    content: msg.text,
                    timestamp: Utc::now(),
                    media: vec![],
                    metadata: Default::default(),
                };
                self.inbound_tx.send(inbound).await?;
            }
        }
    }

    async fn stop(&mut self) -> Result<()> {
        // Cleanup
        Ok(())
    }

    async fn send(&self, response: &Response) -> Result<()> {
        // Send message to platform
        self.platform_client.send(
            &response.chat_id,
            &response.content,
        ).await?;
        Ok(())
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.config.allow_from.is_empty() 
            || self.config.allow_from.contains(&sender_id.to_string())
    }
}
```

### 2. Add Configuration

```rust
// src/config.rs

#[derive(Debug, Clone, Deserialize)]
pub struct MyChannelConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub allow_from: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChannelsConfig {
    pub telegram: TelegramConfig,
    pub whatsapp: WhatsAppConfig,
    pub my_channel: MyChannelConfig,  // Add your channel
}
```

### 3. Register with Channel Manager

```rust
// src/adapters/manager.rs

impl ChannelManager {
    pub fn new(config: &Config, inbound_tx: mpsc::Sender<InboundMessage>) -> Self {
        let mut channels: Vec<Box<dyn Channel>> = vec![];

        if config.channels.telegram.enabled {
            channels.push(Box::new(TelegramChannel::new(...)));
        }
        
        if config.channels.my_channel.enabled {
            channels.push(Box::new(MyChannel::new(
                config.channels.my_channel.clone(),
                inbound_tx.clone(),
            )));
        }

        Self { channels }
    }
}
```

---

## Adding an LLM Provider

### 1. Implement LlmClient Trait

```rust
// src/agent/my_provider.rs

use async_trait::async_trait;
use crate::agent::llm::{LlmClient, LlmResponse, Message, ToolDefinition, LlmConfig};
use crate::error::Result;

pub struct MyProvider {
    api_key: String,
    client: reqwest::Client,
}

#[async_trait]
impl LlmClient for MyProvider {
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        // Call your LLM API
        let response = self.client.post("https://my-llm.example.com/v1/chat")
            .json(&json!({
                "messages": messages,
                "tools": tools,
                "max_tokens": config.max_tokens,
            }))
            .send()
            .await?;
        
        // Parse response...
        Ok(LlmResponse { ... })
    }

    fn default_model(&self) -> &str {
        "my-model-v1"
    }
}
```

---

## Best Practices

1. **Error Handling**: Use `Result<T, Error>` everywhere, never panic
2. **Async**: All I/O operations should be async
3. **Testing**: Every new component needs unit tests
4. **Documentation**: Add rustdoc comments
5. **Configuration**: Make things configurable, not hardcoded
6. **Logging**: Use `tracing` for structured logging

```rust
use tracing::{info, warn, error, debug};

#[instrument(skip(self))]
async fn execute(&self, params: Value) -> Result<String> {
    debug!(tool = %self.name(), "Executing tool");
    // ...
    info!(result_len = result.len(), "Tool completed");
    Ok(result)
}
```
