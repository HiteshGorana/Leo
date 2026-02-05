<div align="center">

# Leo 🦁

![Leo Cover](assets/ghibli_cover.png)

> **Roaringly fast. Delightfully simple. Pure Rust.**
>
> ⚠️ **Beta Status**: Active development. Expect sharp edges! 🦁

</div>

Leo is an ultra-lightweight **AI Personal Assistant** designed to be your own personal companion on any OS and any platform.

## Features

- **🦁 Personal AI**: Runs locally or connects to powerful LLMs like Gemini.
- **🚀 Ultra-fast**: Built in Rust for maximum performance.
- **🔐 Secure Authentication**: Integrated Gemini CLI OAuth support (no loose API keys!).
- **🔌 Extensible**: Modular architecture with support for custom Skills and Tools.
- **💬 Multi-Platform**: Connects to Telegram, acts as a CLI tool, or runs as a background daemon.
- **🧠 Intelligent**: Persistent memory and context management.
- **🛠️ Capable**: Native tools for file editing, searching (grep), git operations, and web access.
- **🛡️ Robust**: Built-in rate limit handling and resilience.
- **⚡ Fast**: Pure Rust implementation.

## Quick Start

### Prerequisites
- Rust (latest stable)
- Gemini CLI (`npm install -g @google/gemini-cli`) - *Recommended for easy auth*

### Installation

#### Option 1: Pre-built Binaries (Recommended)

Download the latest release for your platform from the [Releases Page](https://github.com/HiteshGorana/Leo/releases).

**macOS / Linux:**
1. Download the archive for your architecture (`x86_64` or `aarch64`/Apple Silicon).
2. Extract the binary:
   ```bash
   tar xzf leo-*.tar.gz
   ```
3. Move it to your path:
   ```bash
   sudo mv leo /usr/local/bin/
   ```
   *(On macOS, you may need to allow the app in Security settings if unsigned)*

**Windows:**
1. Download the `.zip` file.
2. Extract `leo.exe`.
3. Add the folder to your system `PATH` or run from PowerShell.

#### Option 2: Build from Source

Clone the repository and build (requires Rust):

```bash
git clone https://github.com/HiteshGorana/Leo.git
cd Leo
cargo build --release
# Binary will be in target/release/leo
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
Start the Telegram gateway (interactive setup on first run):
```bash
cargo run -- gateway
```
The gateway shows a clean single-line log for each message:
```
◆ telegram → Leo → ⚙ tool_name → telegram ✔
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
