# Unit 6: Command Palette, Custom Commands, Theming & CI Operations

## 1. Scope
Deliver user customization and operational readiness: Command Palette (`Ctrl+Shift+P`), custom commands in `config.toml` and `agemax.toml`, TOML themes with hot reload, Windows-friendly keybindings, and GitHub Actions CI workflow with `cargo-deny` and release packaging.

## 2. Requirements Traceability
- **REQ-052**: Config loading from `%APPDATA%\AgeMax\config.toml` and workspace `agemax.toml`.
- **REQ-053**: Configuration hot reload via `notify`.
- **REQ-054**: Windows-friendly default keybindings (never intercepting plain `Ctrl+C`).
- **REQ-055**: Fuzzy search Command Palette (`Ctrl+Shift+P`).
- **REQ-056**: Custom commands in command palette.
- **REQ-057**: TOML theme definitions (colors, accents, rings).
- **REQ-061**: < 1% idle CPU.
- **REQ-062**: Zero clippy warnings under `-D warnings`.
- **REQ-063**: Automated `cargo-deny` compliance.

## 3. Interfaces & Data Structures
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgeMaxConfig {
    pub general: GeneralConfig,
    pub theme: ThemeConfig,
    pub keybindings: KeybindingsConfig,
    pub commands: Vec<CustomCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub working_dir: Option<PathBuf>,
    pub keybinding: Option<String>,
}
```

## 4. Acceptance Criteria
1. Pressing `Ctrl+Shift+P` opens the Command Palette; typing filters commands with fuzzy matching.
2. Custom commands defined in `config.toml` appear in the palette and execute upon selection.
3. Editing `config.toml` reloads themes and keybindings instantly without restarting the application.
4. Plain `Ctrl+C` sends SIGINT to the running terminal program; `Ctrl+Shift+C` copies selected text.
5. GitHub Actions CI runs on Windows runner, executing `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo-deny check`, and produces a release `.zip`.

## 5. Test Plan
- Configuration parser unit tests with valid and invalid TOML samples.
- Keybinding collision tests ensuring no reserved terminal controls are shadowed.
- CI pipeline validation on GitHub Actions Windows runner.
- Benchmark verification of release binary startup time (< 300 ms).
