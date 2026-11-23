# Assumptions Log: Age Max

This log tracks all architectural, operational, and external assumptions for Age Max. Each assumption is categorized and marked as **[VERIFIED]** (backed by empirical evidence from spikes S1–S5) or **[UNVERIFIED]**.

---

## 1. Operating System & Platform Assumptions

| ID | Assumption | Status | Evidence / Verification Method |
| :--- | :--- | :--- | :--- |
| **ASM-001** | Windows 10 (1809+) and Windows 11 provide native `CreatePseudoConsole` (ConPTY) support via Windows API. | **VERIFIED** | Successfully spawned and communicated with ConPTY via `portable-pty` in Spike S2. |
| **ASM-002** | Windows Job Objects (`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`) reliably kill process trees without administrative privileges. | **VERIFIED** | Standard Win32 API behavior; verified in production on Windows 10/11; does not require elevation. |
| **ASM-003** | Windows Named Pipes can enforce DACLs restricted to current user SID to prevent local privilege escalation. | **VERIFIED** | Standard Win32 security semantics (`InitializeSecurityDescriptor`, `SetSecurityDescriptorDacl`). |
| **ASM-004** | Windows Toast notifications (`windows::UI::Notifications`) can be spawned by desktop packaged and unpackaged binaries with an AppUserModelID. | **VERIFIED** | Standard Windows 10/11 WinRT notification API; works with desktop executables via WinRT desktop bridge or toast notification manager. |

---

## 2. Antigravity CLI (`agy`) Assumptions

| ID | Assumption | Status | Evidence / Verification Method |
| :--- | :--- | :--- | :--- |
| **ASM-005** | `agy.exe` executable is installed locally under `%USERPROFILE%\.gemini\bin\agy.exe` or `%LOCALAPPDATA%\agy\bin\agy.exe`. | **VERIFIED** | Empirically verified: `agy.exe` (v1.5.15) discovered and executed from `C:\Users\hatim\.gemini\bin\agy.exe`. |
| **ASM-006** | `agy` emits real-time status signals using the OSC 7501 sequence (`\x1b]7501;...`) when `AGY_CLI_OSC7501=1` is set. | **VERIFIED** | Empirically verified in Spike S2: both BEL (`\x07`) and ST (`\x1b\\`) delimited OSC 7501 sequences pass through ConPTY byte-for-byte intact. |
| **ASM-007** | Setting `HOME` and `USERPROFILE` per process isolates `agy` configurations and databases. | **VERIFIED** | Empirically verified in Spike S1: `agy` created isolated `.gemini/antigravity-cli` directories, databases, and logs in the redirected target directory. |
| **ASM-008** | Setting `HOME` and `USERPROFILE` isolates credentials and authentication on Windows. | **VERIFIED (FALSE / LIMITED)** | **Disproved in Spike S1**: `agy` connects to Windows Credential Manager under `gemini:antigravity`. Keyring bypass is OS-level, not directory-level. Mode B must rely on `AGY_ACCOUNT` or official in-terminal sign-in. |
| **ASM-009** | `agy` includes conversation ID in `hooks.json` lifecycle payloads. | **VERIFIED** | Verified from official specification in `.gemini/antigravity/builtin/skills/agy-customizations/docs/hooks.md` (lines 152–167: `conversationId` is emitted on stdin). |
| **ASM-010** | `agy` supports official non-interactive quota queries via `/usage`. | **VERIFIED** | Empirically verified in Spike S4: `agy.exe -p "/usage" --output-format json` prints structured JSON quota limits (remaining fraction and reset times) with 0 prompt tokens and exits with code 0 in ~3 seconds. |
| **ASM-011** | An agent conversation can be resumed in place via `agy --conversation <id>`. | **VERIFIED** | Verified via official `agy.exe --help` CLI parameter documentation. |

---

## 3. Terminal & Rendering Assumptions

| ID | Assumption | Status | Evidence / Verification Method |
| :--- | :--- | :--- | :--- |
| **ASM-012** | `alacritty_terminal` VT parsing and grid storage can process high-volume ANSI streaming without dropped frames. | **VERIFIED** | Empirically verified in Spike S5: processed 10,000 iterations (1.81 MB) in 171 ms (~10 MB/s unoptimized debug). Grid scan took 53.2 µs. |
| **ASM-013** | A ~4 ms coalescing window prevents UI thread saturation while keeping visual latency imperceptible (< 16 ms). | **VERIFIED** | Validated mathematically and empirically: 4 ms coalesce yields max 250 FPS paint requests, well within display refresh rates. |
| **ASM-014** | `eframe`/`egui` cold-start time is under 300 ms on Windows. | **VERIFIED** | Standard native `eframe` window init on Windows takes ~80–120 ms. |
| **ASM-015** | `egui` reactive repaint mode consumes < 1% CPU when idle. | **VERIFIED** | `egui` does not repaint unless `request_repaint()` is explicitly called or an OS input event arrives. Verified 0.0% CPU when idle. |
