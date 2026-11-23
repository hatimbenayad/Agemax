# ADR-007: Local-Only Named Pipe IPC and Windows Security ACLs

## Status
Accepted

## Context
Age Max exposes a scriptable command-line interface (`agemax CLI`) to query status, split panes, send input, and trigger workflows. Communication must occur locally with high throughput and strict security. The IPC mechanism must prevent unprivileged local users or external network actors from injecting keystrokes or reading screen contents.

## Decision
1. Transport: Windows Named Pipe using the format `\\.\pipe\agemax-<username>`.
2. Access Control: The named pipe server creates a custom Security Descriptor (`SECURITY_ATTRIBUTES`) containing a Discretionary Access Control List (DACL) that grants access exclusively to the current user's Security Identifier (SID) and Local System (`SYSTEM`), denying `Network Logon` and other user accounts on the machine.
3. Protocol: JSON-RPC 2.0 framing over the pipe, serialized via `serde_json`.
4. Platform abstraction: The `agemax-ipc` crate exposes a trait-based interface (`IpcServer`, `IpcClient`) allowing future Unix Domain Socket implementations on Linux/macOS.

## Consequences
- High-speed local RPC without network port contention or firewall prompts.
- Immune to cross-user snooping or unauthorized command injection on multi-user Windows workstations.
