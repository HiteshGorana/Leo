# Migration Plan

## Overview

This document tracks the Python → Rust migration progress. The migration follows an incremental approach, converting one module at a time while preserving behavior.

**Total Python LOC:** ~4,000  
**Target:** Compiling, idiomatic Rust with equivalent functionality

---

## Phase B: Rust Scaffold ✅/🔴

### Milestone B1: Project Setup
- [ ] Create `Cargo.toml` with all dependencies
- [ ] Create `src/lib.rs` (library root)
- [ ] Create `src/main.rs` (CLI entry point)

### Milestone B2: Core Traits
- [ ] Define `LlmClient` trait in `src/agent/llm.rs`
- [ ] Define `Tool` trait in `src/tools/mod.rs`
- [ ] Define `MemoryStore` trait in `src/memory/mod.rs`
- [ ] Define `Skill` trait in `src/skills/mod.rs`
- [ ] Define `Channel` trait in `src/adapters/mod.rs`

### Milestone B3: Test Infrastructure
- [ ] Create `FakeLlmClient` (returns predictable responses)
- [ ] Create `InMemoryStore` (no I/O)
- [ ] Create `DummyToolRunner` (mock execution)
- [ ] Write first unit test for AgentLoop stub
- [ ] `cargo test` passes

---

## Phase C: Incremental Conversion

### C1: Message Types (Priority: Highest)
**Python:** `nanobot/bus/events.py` (38 lines)  
**Rust:** `src/agent/message.rs`

```rust
// Key types to implement:
pub struct InboundMessage { ... }
pub struct Response { ... }
```

**Tests:**
- [ ] Unit test: Create InboundMessage, verify fields
- [ ] Unit test: session_key() generation

---

### C2: Configuration (Priority: High)
**Python:** `nanobot/config/schema.py` (141 lines)  
**Rust:** `src/config.rs`

```rust
// Key types:
pub struct Config { ... }
pub struct ChannelsConfig { ... }
pub struct ProvidersConfig { ... }
```

**Tests:**
- [ ] Unit test: Deserialize sample config.json
- [ ] Unit test: Default values

---

### C3: LLM Provider (Priority: High)
**Python:** `nanobot/providers/base.py` (70 lines)  
**Rust:** `src/agent/llm.rs`

```rust
// Key types:
pub trait LlmClient { ... }
pub struct LlmResponse { ... }
pub struct ToolCallRequest { ... }
```

**Tests:**
- [ ] Unit test: FakeLlmClient returns expected response
- [ ] Unit test: Parse tool calls from response

---

### C4: Tool System (Priority: High)
**Python:** 
- `nanobot/agent/tools/base.py` (103 lines)
- `nanobot/agent/tools/registry.py` (74 lines)

**Rust:** 
- `src/tools/mod.rs` (trait)
- `src/tools/runner.rs` (registry)

```rust
pub trait Tool { ... }
pub struct ToolRunner { ... }
```

**Tests:**
- [ ] Unit test: Register tool, execute
- [ ] Unit test: Validate parameters
- [ ] Port existing Python test: `test_tool_validation.py`

---

### C5: Memory System (Priority: Medium)
**Python:** `nanobot/agent/memory.py` (110 lines)  
**Rust:** `src/memory/store.rs`

```rust
pub trait MemoryStore { ... }
pub struct FileMemoryStore { ... }
pub struct InMemoryStore { ... }  // for tests
```

**Tests:**
- [ ] Unit test: InMemoryStore save/load/search
- [ ] Integration test: FileMemoryStore with temp dir

---

### C6: Skills System (Priority: Medium)
**Python:** `nanobot/agent/skills.py` (229 lines)  
**Rust:** `src/skills/registry.rs`

```rust
pub trait Skill { ... }
pub struct SkillRegistry { ... }
```

**Tests:**
- [ ] Unit test: Load skill from markdown file
- [ ] Unit test: Parse frontmatter metadata
- [ ] Unit test: Check requirements (bins, env)

---

### C7: Context Builder (Priority: Medium)
**Python:** `nanobot/agent/context.py` (218 lines)  
**Rust:** `src/agent/context.rs`

```rust
pub struct ContextBuilder { ... }
```

**Tests:**
- [ ] Unit test: Build system prompt with memory + skills
- [ ] Unit test: Build messages list

---

### C8: Agent Loop (Priority: Core)
**Python:** `nanobot/agent/loop.py` (338 lines)  
**Rust:** `src/agent/loop.rs`

```rust
pub struct AgentLoop<C: LlmClient> { ... }
```

**Tests:**
- [ ] Unit test: Simple conversation (no tools)
- [ ] Unit test: Tool call → result → final response
- [ ] Unit test: Max iterations limit
- [ ] Integration test: Full loop with FakeLlm

---

### C9: Built-in Tools (Priority: Medium)
**Python:** `nanobot/agent/tools/*.py`  
**Rust:** `src/tools/*.rs`

| Tool | Python Lines | Status |
|------|--------------|--------|
| `read_file` | ~30 | 🔴 |
| `write_file` | ~40 | 🔴 |
| `edit_file` | ~50 | 🔴 |
| `list_dir` | ~30 | 🔴 |
| `exec` | ~80 | 🔴 |
| `web_search` | ~60 | 🔴 |
| `web_fetch` | ~60 | 🔴 |
| `message` | ~40 | 🔴 |
| `spawn` | ~50 | 🔴 |

**Tests:**
- [ ] Each tool has unit tests with mocked I/O

---

### C10: Telegram Adapter (Priority: High)
**Python:** `nanobot/channels/telegram.py` (~200 lines)  
**Rust:** `src/adapters/telegram.rs`

**Tests:**
- [ ] Unit test: Parse incoming update
- [ ] Unit test: Format outgoing message
- [ ] Integration: Manual test with real bot

---

### C11: CLI (Priority: Must Have)
**Python:** `nanobot/cli/commands.py` (657 lines)  
**Rust:** `src/main.rs`

Commands to implement:
- [ ] `leo onboard`
- [ ] `leo agent -m "..."`
- [ ] `leo gateway`
- [ ] `leo status`
- [ ] `leo channels status`
- [ ] `leo cron ...`

**Tests:**
- [ ] Integration test: CLI arg parsing
- [ ] Manual: Run each command

---

## Progress Tracking

| Module | Python LOC | Rust Status | Tests |
|--------|-----------|-------------|-------|
| Message types | 38 | 🔴 | 🔴 |
| Config | 141 | 🔴 | 🔴 |
| LLM Provider | 70 | 🔴 | 🔴 |
| Tool trait | 103 | 🔴 | 🔴 |
| ToolRunner | 74 | 🔴 | 🔴 |
| MemoryStore | 110 | 🔴 | 🔴 |
| Skills | 229 | 🔴 | 🔴 |
| Context | 218 | 🔴 | 🔴 |
| AgentLoop | 338 | 🔴 | 🔴 |
| Tools (all) | ~400 | 🔴 | 🔴 |
| Telegram | ~200 | 🔴 | 🔴 |
| ~~WhatsApp~~ | - | ⬜ Skipped | ⬜ |
| ~~Feishu~~ | - | ⬜ Skipped | ⬜ |
| CLI | 657 | 🔴 | 🔴 |

**Legend:**
- 🔴 Not started
- 🟡 In progress  
- 🟢 Complete

---

## Definition of Done

Each module is "done" when:
1. ✅ Rust code compiles without warnings
2. ✅ Matches Python behavior for all cases
3. ✅ Has unit tests passing
4. ✅ Uses idiomatic Rust (Result, Option, traits)
5. ✅ Properly handles errors (no unwrap in library code)
6. ✅ Documented with rustdoc comments
7. ✅ MODULE_MAP.md updated to 🟢 status

---

## Notes

- Dependencies not yet ported should use TODO stubs
- Each conversion should update this file
- Run `cargo clippy` after each module
- Keep commits atomic (one module = one commit)
