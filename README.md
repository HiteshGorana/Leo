# Leo 🦁

![Leo Cover](assets/cover.png)

> **Roaringly fast. Delightfully simple. Pure Rust.**

Leo is an ultra-lightweight personal AI assistant framework designed to be your own personal companion on any OS and any platform. Built with a "pure Rust" philosophy, it prioritizes speed, type safety, and simplicity.

## Features

- **🦁 Personal AI**: Runs locally or connects to powerful LLMs like Gemini.
- **🚀 Ultra-fast**: Built in Rust for maximum performance and minimal footprint.
- **🔐 Secure Authentication**: Integrated Gemini CLI OAuth support (no loose API keys!).
- **🔌 Extensible**: Modular architecture with support for custom Skills and Tools.
- **💬 Multi-Platform**: Connects to Telegram, acts as a CLI tool, or runs as a background daemon.
- **📦 Zero-Config Storage**: Intelligent, file-based memory and session management.

## Quick Start

### Prerequisites
- Rust (latest stable)
- Gemini CLI (`npm install -g @google/gemini-cli`) - *Recommended for easy auth*

### Installation

Clone the repository and build:

```bash
git clone https://github.com/HiteshGorana/Leo.git
cd Leo
cargo build --release
```

### Usage

**1. Onboard**
Initialize your configuration and workspace:
```bash
cargo run -- onboard
```

**2. Login (Recommended)**
Use your existing Gemini CLI credentials for seamless authentication:
```bash
cargo run -- login
```

**3. Chat**
Start an interactive chat session:
```bash
cargo run -- agent
```
Or send a single message:
```bash
cargo run -- agent -m "Write a haiku about Rust"
```

**4. Gateway**
Start the Telegram gateway (requires configuration):
```bash
cargo run -- gateway
```

## Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [Agent Implementation Guide](AGENT.md)
- [Extension Guide (Skills & Tools)](docs/EXTENDING.md)
- [Testing Strategy](docs/TESTING.md)

## Project Structure

```
leo/
├── src/            # Source code
├── docs/           # Documentation
├── tests/          # Integration tests
└── Cargo.toml      # Manifest
```

## License

This project is licensed under the MIT License.
