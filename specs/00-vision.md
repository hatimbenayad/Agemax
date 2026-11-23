# 00 - Product Vision: Age Max (`agemax`)

## 1. Executive Summary

**Age Max** (`agemax`) is a high-performance, Windows-first terminal workspace designed specifically for running, observing, and orchestrating multiple Antigravity CLI (`agy`) agent sessions in parallel. The name signifies *maximizing the agent's capacity*—transforming the AI coding experience from a single serial terminal interaction into a structured, highly visible, multi-threaded mission control center.

## 2. Core Problem Statement

Modern AI coding agents (such as Antigravity CLI, Claude Code, and Codex) require autonomy, frequent command execution, and occasional human-in-the-loop decisions (confirming plans, approving high-risk edits, providing clarification). When developers run multiple agents across different features or repositories:
1. **Window Chaos**: Terminal tabs and windows proliferate without clear status visibility.
2. **Attention Waste**: Developers either hover anxiously waiting for turns to finish, or tab away and forget an agent is blocked waiting for user approval.
3. **Orphaned Processes**: Closing terminals or cancelling tasks frequently leaves zombie child processes (Node, Python, compilers) locking files on Windows.
4. **Context Loss**: Re-opening terminal windows requires manually navigating directories and reconstructing session context.
5. **No Native Windows First-Class Experience**: Existing multiplexers either rely on macOS-exclusive frameworks, heavy webviews (Electron/Tauri) with sluggish input latency, or Unix-centric terminal multiplexers (tmux) that handle Windows ConPTY poorly.

## 3. Product Vision & Positioning

Age Max solves these challenges through a lightweight, 100% native Rust architecture designed ground-up for Windows 10/11:
- **Vertical Hierarchy**: Workspaces (projects/repos) contain Tabs, which split into an arbitrary tree of Panes.
- **Agent Awareness**: Deep, zero-overhead observation of agent states (`Idle`, `Working`, `Waiting for input`, `Done`, `Failed`) using native ConPTY OSC 7501 signaling and bell triggers.
- **Attention System**: Pane colored rings, sidebar unread badges, flashing taskbar icons, and Windows toast notifications allow developers to work elsewhere and jump instantly to the pane that needs them.
- **Zero Credential Boundary**: Age Max never touches, reads, parses, or proxies credentials. The user authenticates directly in official CLI sessions.
- **Windows-Native Process Hygiene**: Child process trees are bound to Windows Job Objects with auto-termination limits (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`).
- **Programmable Control**: A Unix-like named pipe JSON-RPC interface and companion CLI allows scripts, hooks, and automated workflows to control the multiplexer.

## 4. Product Boundaries & Non-Goals

- **No AI Inside Age Max**: Age Max does not bundle LLMs, prompts, or AI runtime code. The intelligence resides entirely in the user's `agy` CLI processes.
- **No Webview Chrome**: The core application is rendered natively with Rust, meeting strict cold-start (< 300 ms) and keystroke-to-glyph (< 16 ms) budgets.
- **No Telemetry**: Completely local, zero phoning home.
- **Dual Licensing**: Dual-licensed under `MIT OR Apache-2.0` with clean-room isolation from copyleft projects.
