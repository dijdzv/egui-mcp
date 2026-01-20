# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

egui-mcp は Claude などの MCP クライアントが egui GUI アプリケーションと対話できるようにするツール。AccessKit 経由で UI ツリーを抽出し、MCP ツールとして公開する。

## Architecture

```
[Claude/MCP Client] --MCP/stdio--> [egui-mcp-server] --IPC/Unix Socket--> [egui app with egui-mcp-client]
```

### Crates

- **egui-mcp-server**: rmcp を使用した MCP サーバーバイナリ。`ping`、`check_connection`、`get_ui_tree` ツールを提供。Unix socket IPC で egui アプリと通信。
- **egui-mcp-client**: egui アプリに組み込むライブラリ。IPC サーバーを実行し、AccessKit の `TreeUpdate` から UI ツリーを抽出。
- **egui-mcp-protocol**: 共有型（`UiTree`、`NodeInfo`、`Request`、`Response`）と IPC 用の length-prefixed メッセージフレーミング。

### IPC Protocol

メッセージは length-prefixed（4バイト big-endian 長 + JSON ペイロード）。ソケットパス: `$XDG_RUNTIME_DIR/egui-mcp.sock` または `/tmp/egui-mcp.sock`。

## Development Environment

- **Ubuntu 24.04+ 必須** (glibc 2.39+ が必要)
- **WSL2で動作**（WSLg必須）
- **依存環境追加時は Nix (devenv) を使用する**

nix + devenv + direnv を使用。プロジェクトディレクトリに `cd` すると direnv が自動ロード。

### システム依存パッケージ (apt)

```bash
sudo apt-get install -y libwayland-dev libxkbcommon-dev fonts-ipafont-gothic
```

### Nix (devenv) が提供するもの

- Rust ツールチェイン (stable)
- pkg-config, libxkbcommon, wayland (ビルド依存)
- nixGL (WSLg OpenGL互換性)
- just, jq, git (開発ツール)

### 環境変数 (devenv が自動設定)

- `WAYLAND_DISPLAY=wayland-0`
- `XDG_RUNTIME_DIR=/mnt/wslg/runtime-dir`
- `MESA_DEBUG=silent`, `LIBGL_DEBUG=quiet`, `EGL_LOG_LEVEL=fatal` (WSLg警告抑制)

## Commands

```bash
just check    # clippy と fmt check を実行
just fmt      # コードをフォーマット
just build    # 全ターゲットをビルド
just test     # テストを実行
just demo     # デモ egui アプリを実行
just server   # MCP サーバーを実行
```

## Testing the Full Flow

1. ターミナル1: `just demo`（IPC サーバー付き egui アプリを起動）
2. ターミナル2: `just server`（MCP サーバーを起動）
3. stdio 経由でサーバーに MCP リクエストを送信

## Key Dependencies

- **rmcp**: Rust MCP SDK。`#[tool_router]` と `#[tool_handler]` マクロを提供
- **accesskit**: egui からアクセシビリティツリーを抽出
- **tokio**: IPC 通信用の async ランタイム
