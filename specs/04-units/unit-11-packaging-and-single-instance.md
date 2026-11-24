# Unit 11: Windows Packaging, Single-Instance IPC Activation & Unit 7 Stretch Hooks

## 1. Scope
1. Embed `.ico` Windows application icon, add Windows installer configuration (Inno Setup / MSI manifest + `winget` metadata), and optional Explorer "Open Folder in Age Max" context menu script.
2. Add single-instance named-pipe detection on `agemax-app.exe` launch (`\\.\pipe\agemax-<username>`): focus existing window and optionally open the passed directory rather than colliding.
3. Follow-up foundation toward [Unit 7 Stretch Features](file:///c:/Users/hatim/Downloads/agemax/specs/04-units/unit-7-stretch-features.md) (`agy acp` visual card panel, WebView2 local preview pane).

## 2. Requirements Traceability
- **REQ-041**–**REQ-044**, **REQ-078**, **REQ-079**.
