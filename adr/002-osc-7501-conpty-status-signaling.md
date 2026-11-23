# ADR-002: Real-time Agent Status Signaling via OSC 7501 on ConPTY

## Status
Accepted (Validated by Spike S2)

## Context
Antigravity CLI agents transition through multiple operational states: `Idle`, `Working`, `Waiting for input`, `Done`, and `Failed`. Age Max needs real-time, low-overhead notifications when an agent requires human attention or finishes a turn. Section 5 notes that `agy` emits OSC 7501 sequences (`AGY_CLI_OSC7501`). We needed to empirically verify whether Windows ConPTY passes OSC 7501 sequences through untouched or strips them.

## Empirical Findings (Spike S2)
1. A ConPTY terminal was spawned using `portable-pty` on Windows 11.
2. Escape sequences were emitted using both BEL (`\x07`) and String Terminator ST (`\x1b\\`):
   - `\x1b]7501;status=working\x07`
   - `\x1b]7501;status=waiting;prompt=approval\x1b\\`
3. Raw byte stream analysis from the master PTY reader confirmed:
   - Both BEL and ST sequences were delivered through ConPTY byte-for-byte intact with zero filtering or corruption.
   - Total latency was immediate (sub-millisecond).

## Decision
1. Set `AGY_CLI_OSC7501=1` and `AGY_CLI_OSC7501_MESSAGES=brief` in the environment of all agent panes launched by Age Max.
2. The terminal VT parser (`agemax-term`) will intercept OSC 7501 sequences directly in the incoming byte stream before updating the terminal grid.
3. Multi-tier status fallback:
   - **Tier 1 (Primary)**: OSC 7501 emitted directly over PTY stream.
   - **Tier 2 (Secondary)**: Bell character (`\x07` / OSC 9 / OSC 99 / OSC 777) notifications.
   - **Tier 3 (Fallback)**: Lifecycle hooks (`hooks.json`).

## Consequences
- Agent status updates occur in real-time with sub-millisecond responsiveness.
- Zero CPU polling or filesystem monitoring required for core status transitions.
