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

## Git Workflow

### Branch Strategy

- **main**: 保護ブランチ。直接 push 禁止、PR 経由のみ
- **feature/xxx**: 新機能開発用
- **fix/xxx**: バグ修正用

### Branch Protection Rules (main)

GitHub で以下を設定済み:
- Require a pull request before merging
- Require status checks to pass (check, test, clippy, fmt)
- Do not allow bypassing the above settings

### Conventional Commits (必須)

このプロジェクトは [Conventional Commits](https://www.conventionalcommits.org/) に従う。
release-plz がコミットメッセージからバージョンを自動判定する。

#### フォーマット

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

#### Types

| Type | 説明 | バージョン |
|------|------|-----------|
| `feat` | 新機能 | MINOR (0.x.0 → 0.x+1.0) |
| `fix` | バグ修正 | PATCH (0.0.x → 0.0.x+1) |
| `docs` | ドキュメントのみ | - |
| `style` | フォーマット変更（コード動作に影響なし） | - |
| `refactor` | リファクタリング（機能変更なし） | - |
| `perf` | パフォーマンス改善 | PATCH |
| `test` | テスト追加・修正 | - |
| `chore` | ビルド、CI、依存関係など | - |

#### Breaking Changes

破壊的変更は `!` を付けるか、footer に `BREAKING CHANGE:` を記載:

```bash
feat!: change API response format

# または
feat: change API response format

BREAKING CHANGE: The response now returns JSON instead of plain text.
```

#### 例

```bash
# 新機能
git commit -m "feat: add hover tool for mouse movement"

# バグ修正
git commit -m "fix: correct element bounds calculation"

# スコープ付き
git commit -m "feat(server): add drag_element tool"

# 破壊的変更
git commit -m "feat!: rename click_at to click_coordinates"
```

### Release Process

1. main に push すると release-plz が自動で Release PR を作成
2. PR には CHANGELOG とバージョン bump が含まれる
3. PR をマージすると自動で:
   - Git tag 作成
   - GitHub Release 作成
   - crates.io に publish
