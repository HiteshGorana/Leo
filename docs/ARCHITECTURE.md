# Architecture

## System Overview

Leo follows a message-driven architecture with clear separation between:
- **Adapters** (I/O): Chat platforms and external services
- **Core** (Logic): Agent loop, context building, tool execution
- **Storage** (State): Memory and session management

```
┌─────────────────────────────────────────────────────────────┐
│                       Chat Platforms                         │
│     ┌──────────────┐              ┌──────────────┐          │
│     │   Telegram   │              │     CLI      │          │
│     └──────┬───────┘              └──────┬───────┘          │
└────────────┼─────────────────────────────┼──────────────────┘
             │                             │
             v                             v
    ┌─────────────────────────────────────────────────────────────────┐
    │                        Message Bus                                │
    │  ┌─────────────┐                           ┌─────────────────┐  │
    │  │  Inbound Q  │ ─────────────────────────>│   Outbound Q    │  │
    │  └─────────────┘                           └─────────────────┘  │
    └────────────────────────────┬────────────────────────────────────┘
                                 │
                                 v
    ┌─────────────────────────────────────────────────────────────────┐
    │                        Agent Loop                                 │
    │                                                                   │
    │  ┌─────────┐     ┌─────────────┐     ┌─────────────────────┐   │
    │  │ Context │────>│  LLM Client │────>│    Tool Runner      │   │
    │  │ Builder │     └─────────────┘     │  ┌────────────────┐ │   │
    │  └────┬────┘           │             │  │ read_file      │ │   │
    │       │                │             │  │ write_file     │ │   │
    │       v                v             │  │ exec           │ │   │
    │  ┌────────────────────────────────┐  │  │ web_search     │ │   │
    │  │        Messages List           │  │  │ web_fetch      │ │   │
    │  │  [system, user, assistant...]  │  │  │ message        │ │   │
    │  └────────────────────────────────┘  │  └────────────────┘ │   │
    │                                       └─────────────────────┘   │
    └─────────────────────────────────────────────────────────────────┘
                                 │
       ┌─────────────────────────┼─────────────────────────┐
       v                         v                         v
┌─────────────┐          ┌─────────────┐          ┌─────────────┐
│   Memory    │          │   Skills    │          │   Session   │
│   Store     │          │  Registry   │          │  Manager    │
└─────────────┘          └─────────────┘          └─────────────┘
```

## Core Components

### 1. Message Types

```rust
/// Inbound message from any chat platform
pub struct InboundMessage {
    pub channel: String,      // "telegram", "whatsapp", "cli"
    pub sender_id: String,    // User identifier
    pub chat_id: String,      // Chat/conversation identifier
    pub content: String,      // Message text
    pub timestamp: DateTime<Utc>,
    pub media: Vec<String>,   // Media file paths/URLs
    pub metadata: HashMap<String, Value>,
}

/// Outbound response to send
pub struct Response {
    pub content: String,
    pub channel: String,
    pub chat_id: String,
    pub media: Vec<String>,
}
```

### 2. LLM Client

Abstraction over LLM providers (OpenRouter, Anthropic, OpenAI, local models):

```rust
pub trait LlmClient: Send + Sync {
    /// Send messages and get response
    async fn chat(
        &self,
        messages: &[Message],
        tools: &[ToolDefinition],
        config: &LlmConfig,
    ) -> Result<LlmResponse>;
    
    fn default_model(&self) -> &str;
}

pub struct LlmResponse {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: String,
    pub usage: Usage,
}
```

### 3. Tool System

Tools are external actions the agent can take:

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;  // JSON Schema
    
    async fn execute(&self, params: Value) -> Result<String>;
    
    /// Validate parameters before execution
    fn validate(&self, params: &Value) -> Result<()>;
}

pub struct ToolRunner {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRunner {
    pub fn register(&mut self, tool: impl Tool + 'static);
    
    pub async fn execute(&self, name: &str, params: Value) -> Result<String>;
    
    pub fn definitions(&self) -> Vec<ToolDefinition>;
}
```

**Built-in Tools:**

| Tool | Purpose |
|------|---------|
| `read_file` | Read file contents |
| `write_file` | Write/create files |
| `edit_file` | Edit existing files |
| `list_dir` | List directory contents |
| `exec` | Execute shell commands |
| `web_search` | Search the web (Brave API) |
| `web_fetch` | Fetch and parse web pages |
| `message` | Send message to chat |
| `spawn` | Launch background subagent |

### 4. Memory System

File-based memory with daily notes and long-term storage:

```rust
pub trait MemoryStore: Send + Sync {
    /// Save a value
    fn save(&self, key: &str, value: &str) -> Result<()>;
    
    /// Load a value
    fn load(&self, key: &str) -> Result<Option<String>>;
    
    /// Search memory
    fn search(&self, query: &str) -> Result<Vec<MemoryEntry>>;
    
    /// Get memory context for prompt
    fn get_context(&self) -> Result<String>;
}

// File structure:
// ~/.leo/workspace/memory/
//   ├── MEMORY.md       (long-term)
//   ├── 2025-02-05.md   (daily notes)
//   └── ...
```

### 5. Skills System

Skills are markdown files that extend agent capabilities:

```rust
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn content(&self) -> &str;
    fn is_always_active(&self) -> bool;
}

pub struct SkillRegistry {
    workspace_skills: PathBuf,  // User skills
    builtin_skills: PathBuf,    // Bundled skills
}

impl SkillRegistry {
    pub fn list_skills(&self) -> Vec<SkillInfo>;
    pub fn load_skill(&self, name: &str) -> Result<Box<dyn Skill>>;
    pub fn get_always_skills(&self) -> Vec<Box<dyn Skill>>;
    pub fn build_summary(&self) -> String;
}
```

### 6. Context Builder

Assembles the system prompt:

```rust
pub struct ContextBuilder {
    memory: Box<dyn MemoryStore>,
    skills: SkillRegistry,
    workspace: PathBuf,
}

impl ContextBuilder {
    /// Build complete system prompt
    pub fn build_system_prompt(&self) -> String {
        // 1. Core identity
        // 2. Bootstrap files (AGENTS.md, SOUL.md, etc.)
        // 3. Memory context
        // 4. Active skills
        // 5. Skills summary (for progressive loading)
    }
    
    /// Build message list for LLM call
    pub fn build_messages(
        &self,
        history: &[Message],
        current: &str,
    ) -> Vec<Message>;
}
```

### 7. Agent Loop

The core processing loop:

```rust
pub struct AgentLoop<C: LlmClient> {
    client: C,
    max_iterations: usize,
}

impl<C: LlmClient> AgentLoop<C> {
    pub async fn run(
        &self,
        message: InboundMessage,
        context: &mut Context,
    ) -> Result<Response> {
        let mut messages = context.build_messages(&message);
        
        for iteration in 0..self.max_iterations {
            let response = self.client.chat(
                &messages,
                &context.tool_runner.definitions(),
                &context.config,
            ).await?;
            
            if response.tool_calls.is_empty() {
                // Done - return final response
                return Ok(Response {
                    content: response.content.unwrap_or_default(),
                    channel: message.channel,
                    chat_id: message.chat_id,
                    media: vec![],
                });
            }
            
            // Execute tools and continue loop
            for call in response.tool_calls {
                let result = context.tool_runner.execute(
                    &call.name,
                    call.arguments,
                ).await;
                messages.push(Message::tool_result(call.id, result));
            }
        }
        
        Ok(Response::timeout_message(&message))
    }
}
```

### 8. Adapters (Channels)

Chat platform integrations:

```rust
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    
    async fn start(&mut self) -> Result<()>;
    async fn stop(&mut self) -> Result<()>;
    async fn send(&self, message: &Response) -> Result<()>;
    
    fn is_allowed(&self, sender_id: &str) -> bool;
}
```

## Data Flow Example

1. User sends "What's the weather?" via Telegram
2. `TelegramChannel` receives it, creates `InboundMessage`, publishes to bus
3. `AgentLoop::run()` is called:
   - `ContextBuilder` assembles system prompt + history
   - LLM is called with messages + tool definitions
   - LLM responds with `web_search` tool call
   - `ToolRunner` executes search, returns results
   - LLM is called again with tool result
   - LLM responds with final answer
4. `Response` is published to outbound queue
5. `TelegramChannel` sends reply to user

## Authentication System

Leo supports OAuth2 authentication via the Gemini CLI's credentials, using Google's Code Assist API.

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Authentication Flow                           │
│                                                                   │
│  ┌─────────────┐    ┌─────────────────┐    ┌────────────────┐  │
│  │ CLI         │───>│ Auth Provider    │───>│ Code Assist    │  │
│  │ Extractor   │    │ (PKCE OAuth)     │    │ API            │  │
│  └─────────────┘    └─────────────────┘    └────────────────┘  │
│        │                    │                       │            │
│        v                    v                       v            │
│  ┌─────────────┐    ┌─────────────────┐    ┌────────────────┐  │
│  │ oauth2.js   │    │ credentials.json │    │ generateContent│  │
│  │ (Gemini CLI)│    │ (~/.leo/)        │    │ /loadCodeAssist│  │
│  └─────────────┘    └─────────────────┘    └────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Components

```rust
/// src/auth/mod.rs - Module structure
pub mod cli_extractor;   // Extract client_id/secret from Gemini CLI
pub mod provider;        // OAuth2 flow with PKCE
pub mod credentials;     // Token persistence
pub mod pkce;            // PKCE code verifier/challenge
pub mod callback_server; // HTTP server for OAuth redirect
```

### OAuth Flow

1. **Credential Extraction** (`cli_extractor.rs`)
   - Finds Gemini CLI installation (Homebrew or npm)
   - Parses `oauth2.js` to extract `client_id` and `client_secret`

2. **PKCE Challenge** (`pkce.rs`)
   - Generates random 43-byte code verifier
   - Creates SHA256 challenge for secure OAuth

3. **Browser Authorization** (`provider.rs`)
   - Opens Google OAuth consent screen
   - User grants permissions (`cloud-platform`, `userinfo.email`, `userinfo.profile`)

4. **Token Exchange** (`callback_server.rs`)
   - Local HTTP server captures OAuth callback
   - Exchanges authorization code for access/refresh tokens

5. **Token Management** (`credentials.rs`)
   - Saves tokens to `~/.leo/credentials.json`
   - Auto-refreshes expired access tokens

### Code Assist API Integration

```rust
/// src/agent/gemini_oauth.rs
const CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";
const CODE_ASSIST_API_VERSION: &str = "v1internal";

impl GeminiOAuthClient {
    /// Discover project via Code Assist API
    async fn get_or_fetch_project_id(&self, access_token: &str) -> Result<String> {
        // Calls loadCodeAssist or onboardUser
    }
    
    /// Generate content via Code Assist API
    async fn chat(&self, messages: &[Message], tools: &[ToolDefinition]) -> Result<LlmResponse> {
        // POST to cloudcode-pa.googleapis.com/v1internal:generateContent
    }
}
```

### Endpoints Used

| Endpoint | Purpose |
|----------|---------|
| `cloudcode-pa.googleapis.com/v1internal:loadCodeAssist` | Discover user's project |
| `cloudcode-pa.googleapis.com/v1internal:onboardUser` | Onboard to free tier |
| `cloudcode-pa.googleapis.com/v1internal:generateContent` | LLM generation |

## Error Handling

- `thiserror` for defining error types
- `anyhow` for error propagation in app code
- Graceful degradation for non-critical failures
- Structured logging with `tracing`

## Configuration

```rust
pub struct Config {
    pub workspace: PathBuf,
    pub model: String,
    pub max_iterations: usize,
    pub providers: ProvidersConfig,
    pub channels: ChannelsConfig,
    pub tools: ToolsConfig,
}
```

Loaded from `~/.leo/config.json` using serde.
