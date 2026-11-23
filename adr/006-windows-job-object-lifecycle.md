# ADR-006: Child Process Lifecycle Management via Windows Job Objects

## Status
Accepted

## Context
When multiplexing multiple agent sessions and terminal shells on Windows, spawned child processes often spawn their own child processes (e.g., `agy` spawning Node.js, Python, compilers, linters, or git subprocesses). On Windows, calling `TerminateProcess` or closing a handle only terminates the root process, frequently leaving orphaned worker processes running in the background, consuming CPU and locking files.

## Decision
1. For every spawned terminal surface or pane, create a Windows Job Object via `CreateJobObjectW`.
2. Configure the Job Object with `JOBOBJECT_EXTENDED_LIMIT_INFORMATION` setting `LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
3. Assign the spawned root child process to the Job Object via `AssignProcessToJobObject` immediately after creation (or create suspended and assign before resume).
4. When a pane is closed, or when the Age Max application terminates (normally or abruptly), the Job Object handle is closed by the OS, guaranteeing that the entire process hierarchy is immediately and cleanly killed.

## Consequences
- Zero orphaned background processes, compilers, or child agents.
- Clean resource reclamation on Windows without manual process tree traversal.
