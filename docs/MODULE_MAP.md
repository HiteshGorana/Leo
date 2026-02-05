# Module Mapping: Python (nanobot) → Rust (Leo)

This document maps each Python module to its corresponding Rust implementation.

## Core Agent

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/agent/__init__.py` | 10 | `src/agent/mod.rs` | 🔴 Pending |
| `nanobot/agent/loop.py` | 338 | `src/agent/loop.rs` | 🔴 Pending |
| `nanobot/agent/context.py` | 218 | `src/agent/context.rs` | 🔴 Pending |
| `nanobot/agent/memory.py` | 110 | `src/memory/store.rs` | 🔴 Pending |
| `nanobot/agent/skills.py` | 229 | `src/skills/registry.rs` | 🔴 Pending |
| `nanobot/agent/subagent.py` | 243 | `src/agent/subagent.rs` | 🔴 Pending |

## Message Types

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/bus/events.py` | 38 | `src/agent/message.rs` | 🔴 Pending |
| `nanobot/bus/queue.py` | 82 | `src/agent/bus.rs` | 🔴 Pending |

## Tools

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/agent/tools/__init__.py` | 5 | `src/tools/mod.rs` | 🔴 Pending |
| `nanobot/agent/tools/base.py` | 103 | `src/tools/mod.rs` (trait) | 🔴 Pending |
| `nanobot/agent/tools/registry.py` | 74 | `src/tools/runner.rs` | 🔴 Pending |
| `nanobot/agent/tools/filesystem.py` | ~150 | `src/tools/filesystem.rs` | 🔴 Pending |
| `nanobot/agent/tools/shell.py` | ~100 | `src/tools/shell.rs` | 🔴 Pending |
| `nanobot/agent/tools/web.py` | ~120 | `src/tools/web.rs` | 🔴 Pending |
| `nanobot/agent/tools/message.py` | ~50 | `src/tools/message.rs` | 🔴 Pending |
| `nanobot/agent/tools/spawn.py` | ~60 | `src/tools/spawn.rs` | 🔴 Pending |

## LLM Providers

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/providers/__init__.py` | 5 | N/A | N/A |
| `nanobot/providers/base.py` | 70 | `src/agent/llm.rs` (trait) | 🔴 Pending |
| ~~`nanobot/providers/litellm_provider.py`~~ | - | *Not porting (using direct Gemini)* | ⬜ Skipped |
| `nanobot/providers/gemini.rs` | NEW | `src/agent/gemini.rs` | 🔴 Pending |
| ~~`nanobot/providers/transcription.py`~~ | - | *Not porting* | ⬜ Skipped |

## Chat Adapters (Channels)

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/channels/__init__.py` | 5 | N/A | N/A |
| `nanobot/channels/base.py` | 122 | `src/adapters/mod.rs` (trait) | 🔴 Pending |
| `nanobot/channels/manager.py` | ~100 | `src/adapters/manager.rs` | 🔴 Pending |
| `nanobot/channels/telegram.py` | ~200 | `src/adapters/telegram.rs` | 🔴 Pending |
| ~~`nanobot/channels/whatsapp.py`~~ | - | *Not porting* | ⬜ Skipped |
| ~~`nanobot/channels/feishu.py`~~ | - | *Not porting* | ⬜ Skipped |

## Configuration

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/config/__init__.py` | 5 | N/A | N/A |
| `nanobot/config/loader.py` | ~80 | `src/config.rs` | 🔴 Pending |
| `nanobot/config/schema.py` | 141 | `src/config.rs` | 🔴 Pending |

## CLI

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/cli/__init__.py` | 5 | N/A | N/A |
| `nanobot/cli/commands.py` | 657 | `src/main.rs` | 🔴 Pending |

## Cron/Heartbeat

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/cron/service.py` | ~150 | `src/cron/service.rs` | 🔴 Pending |
| `nanobot/cron/types.py` | ~80 | `src/cron/types.rs` | 🔴 Pending |
| `nanobot/heartbeat/service.py` | ~100 | `src/heartbeat.rs` | 🔴 Pending |

## Session Management

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/session/manager.py` | ~120 | `src/session.rs` | 🔴 Pending |

## Utilities

| Python File | Lines | Rust File | Status |
|-------------|-------|-----------|--------|
| `nanobot/utils/helpers.py` | ~50 | Various modules | 🔴 Pending |

## Status Legend

- 🔴 **Pending**: Not started
- 🟡 **Stub**: Skeleton/trait defined
- 🟢 **Complete**: Fully ported and tested

## Conversion Order

Recommended order based on dependencies:

1. **Foundation Layer**
   - `bus/events.py` → Message types
   - `config/schema.py` → Config structs
   - `providers/base.py` → LlmClient trait

2. **Tool Layer**
   - `tools/base.py` → Tool trait
   - `tools/registry.py` → ToolRunner

3. **Memory Layer**
   - `agent/memory.py` → MemoryStore

4. **Skills Layer**
   - `agent/skills.py` → SkillRegistry

5. **Context Layer**
   - `agent/context.py` → ContextBuilder

6. **Agent Core**
   - `agent/loop.py` → AgentLoop

7. **Adapters**
   - `channels/base.py` → Channel trait
   - `channels/telegram.py` (most used)

8. **CLI**
   - `cli/commands.py` → main.rs

9. **Optional Services**
   - Cron, Heartbeat, Session
