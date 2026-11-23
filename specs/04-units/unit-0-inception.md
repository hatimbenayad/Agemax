# Unit 0: Inception, Spikes & Project Skeleton

## 1. Scope
Establish the architectural foundation, licensing boundaries, clean-room standards, and run empirical spikes S1–S5 to eliminate technical unknowns. Create the Cargo workspace skeleton and define all engineering specifications before production construction begins.

## 2. Requirements Traceability
- **REQ-062**: Clean compilation under stable Rust 2021 edition with zero warnings.
- **REQ-063**: Dependency licenses strictly enforced via `cargo-deny` (`MIT OR Apache-2.0`).

## 3. Interfaces & Components
- Root `Cargo.toml` with 7 workspace members: `agemax-core`, `agemax-term`, `agemax-agy`, `agemax-ipc`, `agemax-config`, `agemax-app`, `agemax-cli`.
- `AGENTS.md` containing binding rules, clean-room rules, performance budgets, and coding conventions.
- Complete `/specs` suite: vision, requirements, user stories, architecture, assumptions, unit specs.
- Complete `/adr` suite: ADR-001 through ADR-007.

## 4. Acceptance Criteria
1. Cargo workspace compiles cleanly (`cargo check --workspace` passes).
2. Spikes S1–S5 executed and recorded with empirical evidence in `/spikes` and `/adr`.
3. `cargo-deny` configuration verified against permissive open-source licenses.
4. Formal human approval obtained before moving to Unit 1.

## 5. Test Plan
- Run `cargo check --workspace` to ensure all crate dependencies and traits resolve.
- Run `cargo test --workspace` on stub unit tests.
- Verify git status is clean and structured.
