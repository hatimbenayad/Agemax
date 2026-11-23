# AGENTS.md — Development Guidelines & Constraints for Age Max

This document defines the binding engineering rules, architectural constraints, clean-room standards, and coding conventions for **Age Max** (`agemax`). All contributors and AI agents working on this codebase must strictly adhere to these directives.

---

## 1. Context and Ownership

- **Original Project**: Age Max is the human user's own original project in their own repository. It is **not** a fork, derivative, or upstream contribution of any other software.
- **Inspiration**: Conceptually inspired by the workflow of vertical agent terminal multiplexers (e.g., cmux for macOS).
- **Clean-Room Engineering Rule (MANDATORY)**:
  - Do **NOT** copy, translate, decompile, or paraphrase code, syntax trees, shaders, or visual assets from `cmux` (which is GPL-3.0) or any other copyleft project.
  - Implement all architectures, state machines, layout trees, protocols, and terminal integrations strictly from first principles and the functional specifications in `/specs`.
  - All original Age Max code is dual-licensed under **`MIT OR Apache-2.0`**.
  - License compliance is strictly enforced in CI via `cargo-deny`. Any non-permissive dependency (GPL, AGPL, SSPL, etc.) will fail the build.

---

## 2. Hard Architectural & Safety Constraints

1. **Language & Toolchain**:
   - Rust stable toolchain, 2021 edition or newer.
   - **Strictly No Electron, No Tauri, and No Webview** for the primary application chrome and terminal emulator. The core multiplexer must be native Rust.
2. **Platform Focus**:
   - Primary target: **Windows 10 (1809+, build 17763+) and Windows 11**. ConPTY (`CreatePseudoConsole`) is the native terminal abstraction.
   - All Windows-specific system calls (ConPTY, Named Pipes, Job Objects, Toast notifications, Taskbar flash) must be encapsulated behind platform-agnostic traits to allow future Linux/macOS support.
3. **Strict AI Boundary**:
   - **No embedded AI models, LLM runtimes, or prompts inside Age Max.**
   - Age Max is an agent host, terminal multiplexer, supervisor, and observer. The intelligence belongs solely to the user's CLI processes (e.g., `agy`).
4. **Credential & Security Boundary**:
   - **Zero Credential Access**: Age Max must **never** read, inspect, parse, persist, log, proxy, or transmit user credentials, OAuth tokens, API keys, or keyring secrets.
   - Age Max only launches official binaries (such as `agy.exe`) in configured environments and delegates all authentication flows directly to the official tools.
   - Local-only IPC: Named pipes (`\\.\pipe\agemax-<username>`) must have explicit DACLs restricting access exclusively to the current Windows logon session / user SID.
   - **Windows Job Objects**: Every spawned child process (shells, agent CLIs) must be assigned to a Windows Job Object configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` to guarantee that closing a pane or terminating the multiplexer forcefully terminates the entire child process tree, preventing orphaned background tasks.
5. **Performance Budgets**:
   - **Cold Start**: < 300 ms from process launch to first rendered frame.
   - **Input Latency**: < 16 ms from keystroke to glyph render.
   - **Throughput**: 10 simultaneous panes streaming dense VT output with zero dropped frames and no UI thread stutter.
   - **Idle Footprint**: < 1% CPU utilization when all panes are idle.
   - **PTY Batching**: VT parsing and buffer dirtying must coalesce high-frequency PTY reads (e.g., ~4 ms window) to avoid rendering on every raw byte.
   - **Zero Allocations in Hot Paths**: Keystroke handling and per-character rendering must avoid heap allocations.

---

## 3. Methodology: AI-DLC + SDD

Development follows the **AI-Driven Development Lifecycle (AI-DLC)** paired with **Spec-Driven Development (SDD)**:

1. **Inception Phase**:
   - Formal specifications in `/specs/`:
     - `00-vision.md`
     - `01-requirements.md` (atomic, testable requirements: `REQ-001`, `REQ-002`, etc.)
     - `02-user-stories.md`
     - `03-architecture.md` (crate boundaries, data flow, threading model, Mermaid diagrams)
     - `04-units/` (per-unit implementation specifications: unit-0 through unit-7)
     - `assumptions.md` (every assumption cataloged and verified)
     - `adr/` (Architecture Decision Records: NNN-*.md)
   - Mandatory empirical spikes (S1 through S5) validate all technical uncertainties *before* construction.
   - Formal human sign-off is required before any construction code is authored.
2. **Construction Phase (Unit-by-Unit)**:
   - For each unit: Spec review -> Test implementation (TDD) -> Production code -> Verification (`cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`) -> Specification synchronization -> Demo summary.
   - Every unit must produce a green, passing, shippable state.
   - Never skip gates or proceed to subsequent units without explicit human acknowledgment.
3. **Spec Integrity**:
   - Specs are the single source of truth. Any behavior absent from specs is considered a defect. Every requirement (`REQ-xxx`) must trace directly to automated tests.

---

## 4. Coding & Engineering Conventions

- **Rust Style**:
  - Idiomatic Rust formatted strictly with standard `rustfmt`.
  - Zero warnings under `cargo clippy --all-targets --all-features -- -D warnings`.
- **Error Handling**:
  - Library crates (`agemax-core`, `agemax-term`, `agemax-agy`, `agemax-ipc`, `agemax-config`) use `thiserror` for typed, descriptive domain errors.
  - Application bins (`agemax-app`, `agemax-cli`) may use `anyhow` for top-level context propagation.
  - Never use `.unwrap()` or `.expect()` in non-test production code unless preceded by an invariant proof comment.
- **Concurrency & Threading**:
  - Decouple PTY I/O from UI rendering: dedicated background threads per PTY pipe feed crossbeam/tokio bounded channels.
  - Coalesce terminal state updates before waking the UI event loop.
  - Heavy background queries (e.g., Git branch, port scanning) must run on throttled worker threads and never block the UI thread.
