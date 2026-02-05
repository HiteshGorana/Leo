# AGENT.md - Leo Rust Implementation Guide

## Overview

**Leo** — Roaringly fast. Delightfully simple. Pure Rust. 🦁

An **Ultra-lightweight AI Personal Assistant**.

This document describes the Rust implementation architecture following the pattern:

```
Chat Apps → Message → Agent Loop (LLM ↔ Tools) → Response
                            ↑
                      Context Layer
                    (Memory + Skills)
```

## Core Architecture

### Message Flow

1. **Inbound**: Chat adapter receives message → `InboundMessage` → MessageBus
2. **Processing**: `AgentLoop` takes message + context → calls LLM → executes tools → loops
3. **Outbound**: `Response` → MessageBus → Chat adapter sends reply

### Core Traits

```rust
/// LLM provider abstraction (swappable: OpenAI, Anthropic, local models)
pub trait LlmClient: Send + Sync {
    async fn chat(&self, messages: &[Message], tools: &[ToolDef]) -> Result<LlmResponse>;
    fn default_model(&self) -> &str;
}

/// Memory persistence layer
pub trait MemoryStore: Send + Sync {
    fn save(&self, key: &str, value: &str) -> Result<()>;
    fn load(&self, key: &str) -> Result<Option<String>>;
    fn search(&self, query: &str) -> Result<Vec<String>>;
}

/// Agent skill (extends capabilities via markdown instruction files)
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn content(&self) -> &str;
}

/// Tool that agent can call
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;  // JSON Schema
    async fn execute(&self, params: serde_json::Value) -> Result<String>;
}
```

### Context

The `Context` struct owns:
- `MemoryStore` - persistent and session memory
- `SkillRegistry` - loaded skills
- `ToolRunner` - tool execution
- `Config` - runtime configuration

### Agent Loop

```rust
pub struct AgentLoop<C: LlmClient> {
    client: C,
    max_iterations: usize,
}

impl<C: LlmClient> AgentLoop<C> {
    pub async fn run(&self, msg: Message, ctx: &mut Context) -> Result<Response> {
        // 1. Build messages from context (system prompt + history + current)
        // 2. Loop: LLM call → tool execution → repeat until done
        // 3. Return final response
    }
}
```

## Project Structure

```
leo/
├── AGENT.md                 # This file
├── Cargo.toml              # Workspace manifest
├── src/
│   ├── main.rs             # CLI entry point (clap)
│   ├── lib.rs              # Library root
│   ├── agent/
│   │   ├── mod.rs          # Agent module exports
│   │   ├── loop.rs         # AgentLoop implementation
│   │   ├── message.rs      # InboundMessage, Response types
│   │   ├── context.rs      # Context builder
│   │   ├── llm.rs          # LlmClient trait + implementations
│   │   ├── gemini.rs       # API key-based Gemini client
│   │   └── gemini_oauth.rs # OAuth-based Gemini client
│   ├── auth/
│   │   ├── mod.rs          # Auth module exports
│   │   ├── provider.rs     # GeminiAuthProvider (OAuth flow)
│   │   ├── credentials.rs  # Token storage/loading
│   │   ├── cli_extractor.rs# Extract creds from Gemini CLI
│   │   ├── pkce.rs         # PKCE code generation
│   │   └── callback_server.rs # HTTP server for OAuth callback
│   ├── memory/
│   │   ├── mod.rs          # MemoryStore trait
│   │   ├── store.rs        # File-based store
│   │   └── in_memory.rs    # In-memory store (for tests)
│   ├── skills/
│   │   ├── mod.rs          # Skill trait
│   │   └── registry.rs     # SkillRegistry
│   ├── tools/
│   │   ├── mod.rs          # Tool trait
│   │   ├── runner.rs       # ToolRunner (executes tools)
│   │   ├── filesystem.rs   # read/write/edit/list tools
│   │   ├── shell.rs        # exec tool
│   │   └── web.rs          # search/fetch tools
│   └── adapters/
│       ├── mod.rs          # Channel trait
│       └── telegram.rs     # Telegram bot
├── docs/
│   ├── ARCHITECTURE.md     # Detailed architecture
│   ├── MODULE_MAP.md       # Python → Rust mapping
│   ├── MIGRATION_PLAN.md   # Conversion roadmap
│   ├── EXTENDING.md        # How to add skills/tools
│   └── TESTING.md          # Testing strategy
└── tests/
    └── integration/        # Integration tests
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `thiserror` | Error type definitions |
| `anyhow` | Error propagation |
| `tracing` + `tracing-subscriber` | Structured logging |
| `serde` + `serde_json` | Serialization |
| `tokio` | Async runtime |
| `clap` | CLI parsing |
| `reqwest` | HTTP client |
| `uuid` | Session ID generation |
| `sha2` + `base64` + `rand` | OAuth PKCE cryptography |
| `open` | Browser opening for OAuth |
| `regex` | Credential extraction |

## Authentication

Leo supports two authentication methods for Gemini:

### Option 1: OAuth via Gemini CLI (Recommended)

Uses credentials from the installed Gemini CLI - no API key needed!

```bash
# 1. Install Gemini CLI (if not already)
brew install gemini-cli

# 2. Login via nanobot
cargo run -- login

# 3. Use the agent
cargo run -- agent -m "Hello!"
```

**How it works:**
- Extracts OAuth credentials from Gemini CLI installation
- Uses Google's Code Assist API (`cloudcode-pa.googleapis.com`)
- Same authentication flow as the official Gemini CLI
- Tokens are cached in `~/.leo/credentials.json`

**Architecture:**

```rust
// src/auth/mod.rs
pub mod cli_extractor;  // Extract creds from Gemini CLI
pub mod provider;       // OAuth flow with PKCE
pub mod credentials;    // Token management
pub mod pkce;           // PKCE code generation
pub mod callback_server;// HTTP server for OAuth callback

// src/agent/gemini_oauth.rs
pub struct GeminiOAuthClient {
    auth_provider: GeminiAuthProvider,
    model: String,
    client: Client,
    project_id: Option<String>,
    session_id: String,
}
```

### Option 2: API Key

Get an API key from [Google AI Studio](https://aistudio.google.com/app/apikey).

```json
// ~/.leo/config.json
{
  "gemini_api_key": "YOUR_API_KEY",
  "provider": "gemini"
}
  "provider": "gemini"
}
```

## Telegram Setup

To use the Telegram adapter:

1. **Create a Bot**:
   - Open Telegram and search for **@BotFather**.
   - Send `/newbot`.
   - Follow instructions to choose a name and username (must end in `bot`).
   - Copy the HTTP API Token (e.g., `123456789:ABCdefGHIjklMNOpqrsTUVwxyz`).

2. **Configure Leo**:
   - Run `cargo run -- gateway` and follow the interactive setup.
   - Or manually edit `~/.leo/config.json`:
     ```json
     {
       "telegram": {
         "enabled": true,
         "token": "YOUR_BOT_TOKEN",
         "allow_from": []
       }
     }
     ```
   - **allow_from**: Empty list allows all users. Add Telegram usernames to restrict access.

3. **Run the Gateway**:
   ```bash
   cargo run -- gateway
   ```

4. **Console Logging**:
   The gateway shows a clean single-line log for each message:
   ```
   ◆ telegram → Leo → ⚙ read_file → telegram ✔
   ```
   - `◆` Message received
   - `⚙ tool_name` Each tool executed
   - `✔` Response sent successfully
   - `✖` Error occurred

## UI Logging

Leo uses a minimal, clean logging style:

| Symbol | Meaning |
|--------|---------|
| `→` | Step/progress |
| `✔` | Success |
| `!` | Warning |
| `✖` | Error |
| `⚙` | Tool execution |
| `◆` | Channel message received |


## Quick Start

```bash
# Build
cargo build

# Run tests
cargo test

# Login (OAuth)
cargo run -- login

# Run CLI
cargo run -- agent -m "Hello!"

# Run gateway
cargo run -- gateway
```

## Testing

All core logic is designed to be deterministic and testable:

- `FakeLlmClient` - returns predictable responses
- `InMemoryStore` - no filesystem access
- `DummyToolRunner` - mock tool execution

```rust
#[tokio::test]
async fn test_agent_loop() {
    let client = FakeLlmClient::new(vec!["Hello, human!"]);
    let mut ctx = Context::new_test();
    let loop_ = AgentLoop::new(client, 10);
    
    let response = loop_.run(Message::user("Hi"), &mut ctx).await?;
    assert_eq!(response.content, "Hello, human!");
}
```

## Design Principles

1. **Core logic is pure**: IO isolated in adapters/tools
2. **Testable by default**: All traits have test implementations
3. **Incremental migration**: Python behavior preserved module-by-module
4. **Minimal dependencies**: Only what's needed
5. **Idiomatic Rust**: Proper error handling, ownership, traits
