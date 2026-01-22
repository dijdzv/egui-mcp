# egui-mcp

An MCP (Model Context Protocol) server that enables AI agents to interact with egui GUI applications.

## Overview

egui-mcp provides UI automation capabilities for [egui](https://github.com/emilk/egui) applications through the Model Context Protocol. It leverages Linux's AT-SPI (Assistive Technology Service Provider Interface) to access the UI tree and exposes it to any MCP-compatible client.

## Features

### Implemented

| Tool | Description | Method |
|------|-------------|--------|
| `get_ui_tree` | Get the complete UI tree | AT-SPI |
| `find_by_label` | Search elements by label (substring match) | AT-SPI |
| `find_by_label_exact` | Search elements by label (exact match) | AT-SPI |
| `find_by_role` | Search elements by role (Button, TextInput, etc.) | AT-SPI |
| `get_element` | Get a specific element by ID | AT-SPI |
| `click_element` | Click element by ID | AT-SPI Action |
| `set_text` | Input text to text fields | AT-SPI EditableText |
| `take_screenshot` | Capture application screenshot | IPC |
| `ping` | Verify server is running | - |
| `check_connection` | Check connection to egui app | IPC |

### Planned

| Tool | Description | Method |
|------|-------------|--------|
| `click_at` | Click at coordinates | IPC |
| `keyboard_input` | Send keyboard input | IPC |
| `scroll` | Scroll operation | IPC |

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                    MCP Client (AI Agent)                      │
└───────────────────────────────────────────────────────────────┘
                              │
                              │ MCP Protocol (stdio)
                              ▼
┌───────────────────────────────────────────────────────────────┐
│                      egui-mcp-server                          │
│                                                               │
│  ┌─────────────────────┐      ┌─────────────────────┐         │
│  │   AT-SPI Client     │      │    IPC Client       │         │
│  │ (UI tree & actions) │      │   (screenshots)     │         │
│  └──────────┬──────────┘      └───────────┬─────────┘         │
└─────────────┼─────────────────────────────┼───────────────────┘
              │ D-Bus                       │ Unix Socket
              ▼                             ▼
┌─────────────────────────┐     ┌───────────────────────────────┐
│      AT-SPI Bus         │     │     egui-mcp-client           │
│  (org.a11y.atspi.*)     │     │  (embedded in egui app)       │
└─────────────────────────┘     └───────────────────────────────┘
              ▲                             ▲
              │ auto-publish                │ embedded
              │                             │
┌───────────────────────────────────────────────────────────────┐
│                      egui Application                         │
│           enable_accesskit() → AccessKit → AT-SPI             │
└───────────────────────────────────────────────────────────────┘
```

## Requirements

- **Linux** (AT-SPI required)
- **Rust** 1.85+ (edition 2024)

## Installation

### Build from Source

```bash
git clone https://github.com/dijdzv/egui-mcp.git
cd egui-mcp
cargo build --release
```

The server binary will be at `target/release/egui-mcp-server`.

## Usage

### 1. Prepare Your egui Application

Add `egui-mcp-client` to enable screenshot support:

```toml
# Cargo.toml
[dependencies]
egui-mcp-client = { git = "https://github.com/dijdzv/egui-mcp.git" }
```

```rust
use egui_mcp_client::McpClient;

fn main() {
    let mcp_client = McpClient::new();

    // Start IPC server
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let client_clone = mcp_client.clone();
    runtime.spawn(async move {
        egui_mcp_client::IpcServer::run(client_clone).await.ok();
    });

    // Run eframe app
    eframe::run_native("My App", options, Box::new(|cc| {
        // Enable AccessKit (publishes UI tree to AT-SPI)
        cc.egui_ctx.enable_accesskit();
        // ...
    }));
}
```

**Note**: Calling `enable_accesskit()` automatically publishes the UI tree via AT-SPI. No manual export is required.

### 2. Configure MCP Client

Add the server to your MCP client's configuration. The format depends on your MCP client:

```json
{
  "mcpServers": {
    "egui-mcp": {
      "command": "/path/to/egui-mcp-server",
      "args": []
    }
  }
}
```

### 3. Available Tools

Once connected, the following MCP tools are available:

- **`ping`** - Check if server is running
- **`check_connection`** - Verify connection to egui application
- **`get_ui_tree`** - Get complete UI structure as JSON
- **`find_by_label`** - Find elements containing a label substring
- **`find_by_label_exact`** - Find elements with exact label match
- **`find_by_role`** - Find elements by role (Button, TextInput, CheckBox, etc.)
- **`get_element`** - Get element details by ID
- **`click_element`** - Click an element by ID (uses AT-SPI Action)
- **`set_text`** - Set text content of a text input by ID (uses AT-SPI EditableText)
- **`take_screenshot`** - Capture screenshot (returns base64 PNG)

## Development

### Setup

This project uses [devenv](https://devenv.sh/):

```bash
cd egui-mcp
# direnv auto-loads the environment, or:
devenv shell
```

### Building the Demo App

The demo app requires additional system dependencies for eframe/egui (Wayland backend):

```bash
# Ubuntu/Debian
sudo apt-get install -y libwayland-dev libxkbcommon-dev
```

**Note**: These are required for building egui/eframe applications with Wayland support, not for egui-mcp-server itself.

### Commands

```bash
just check    # Run clippy and fmt check
just fmt      # Format code
just build    # Build all targets
just test     # Run tests
just demo     # Run demo egui app
just server   # Run MCP server
```

### Testing

1. Terminal 1: `just demo` (start demo app)
2. Terminal 2: `just server` (start MCP server)
3. Connect with any MCP client

## Project Structure

```
egui-mcp/
├── crates/
│   ├── egui-mcp-server/    # MCP server binary
│   ├── egui-mcp-client/    # Library for egui apps
│   └── egui-mcp-protocol/  # Shared protocol definitions
├── examples/
│   └── demo-app/           # Demo egui application
├── devenv.nix              # Development environment
└── justfile                # Build commands
```

## License

MIT OR Apache-2.0

## Related

- [egui](https://github.com/emilk/egui) - Immediate mode GUI library for Rust
- [AccessKit](https://github.com/AccessKit/accesskit) - Cross-platform accessibility abstraction
- [MCP](https://modelcontextprotocol.io/) - Model Context Protocol
- [AT-SPI](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/) - Linux accessibility interface
