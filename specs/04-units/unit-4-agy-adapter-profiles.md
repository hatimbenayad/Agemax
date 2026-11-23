# Unit 4: Antigravity CLI Adapter, Profiles & Quota Management

## 1. Scope
Deliver the Antigravity integration layer (`agemax-agy`): agent binary discovery, launch parameter configuration, Mode A (default zero-setup) and Mode B (multi-account Profiles), zero-credential enforcement, conversation ID capture, and official quota query service.

## 2. Requirements Traceability
- **REQ-031**: `AgentAdapter` trait abstraction.
- **REQ-032**: Auto-discovery of `agy.exe` binary.
- **REQ-033**: Execution flags (`AGY_CLI_OSC7501=1`, `AGY_CLI_DISABLE_AUTO_UPDATE=true`, etc.).
- **REQ-034**: Mode A default execution.
- **REQ-035**: Mode B Profile directories under `%LOCALAPPDATA%\AgeMax\profiles\<name>\`.
- **REQ-036**: Mode B shared history (NTFS junctions) vs isolated mode.
- **REQ-037**: Strict zero-credential access guarantee.
- **REQ-038**: In-terminal interactive sign-in flow.
- **REQ-039**: Quota visibility via `agy -p "/usage" --output-format json`.
- **REQ-040**: Multi-tier conversation ID capture (`SessionStart` hook, cache, sqlite).

## 3. Interfaces & Data Structures
```rust
pub trait AgentAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn detect_binary(&self) -> Option<PathBuf>;
    fn build_command(&self, profile: &ProfileConfig, working_dir: &Path) -> Result<CommandBuilder, AdapterError>;
    fn capture_conversation_id(&self, working_dir: &Path, pid: u32) -> Option<String>;
    fn fetch_quota(&self, profile: &ProfileConfig) -> Result<QuotaSummary, AdapterError>;
}

pub struct ProfileConfig {
    pub name: String,
    pub account_override: Option<String>,
    pub isolate_storage: bool,
    pub profile_dir: PathBuf,
}

pub struct QuotaSummary {
    pub gemini_weekly_remaining: f32,
    pub gemini_5h_remaining: f32,
    pub third_party_weekly_remaining: f32,
    pub third_party_5h_remaining: f32,
    pub next_reset: std::time::SystemTime,
}
```

## 4. Acceptance Criteria
1. `agemax-agy` automatically finds `agy.exe` on Windows without manual path configuration.
2. Launching an agent in Mode A starts `agy` instantly with official keyring authentication.
3. Mode B profile creation initializes `%LOCALAPPDATA%\AgeMax\profiles\<name>\` and applies junctioning if configured.
4. Age Max source code contains zero credential-reading or token-parsing logic.
5. `fetch_quota` executes `agy -p "/usage" --output-format json` in a background thread and parses quota fractions and reset timestamps accurately.
6. Conversation IDs are captured accurately upon session launch and recorded in pane metadata.

## 5. Test Plan
- Unit tests with a fake `agy` executable emitting mock OSC 7501 and hook payloads.
- Quota JSON parser unit tests against real JSON payloads captured in Spike S4.
- NTFS junction creation and cleanup test in temporary test folders.
- Audit test ensuring no credential files are accessed or logged.
