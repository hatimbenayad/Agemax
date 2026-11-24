# Unit 10: Zero-CLI Onboarding, Visual Agent Presets & Settings UI

## 1. Scope
Remove remaining command-line guesswork for new users while honoring the strict no-embedded-AI and zero-credential rules:
1. Asynchronous startup preflight check (`AntigravityAdapter::detect_binary()` + shell check) with a clear onboarding banner if `agy` isn't installed yet.
2. Visual **Agent Launcher** modal providing plain-English presets (*New Session*, *Safe Plan Mode*, *Continue Last*, *Pick Profile*) mapped to `LaunchCommand` cleanly.
3. Full **Settings (`⚙`) Modal** for UI mode, theme presets, font size, default shell, notification preferences, interactive keybinding capture, and custom Quick Actions, saving to `%APPDATA%\AgeMax\config.toml`.

## 2. Requirements Traceability
- **REQ-031**–**REQ-039**, **REQ-052**–**REQ-057**, **REQ-075**–**REQ-077**.

## 3. Acceptance Criteria
1. Missing `agy.exe` shows clear setup guidance without blocking normal terminal use.
2. Launching *Safe Plan Mode* starts `agy --mode plan` inside the pane without reading credentials or wrapping authentication.
3. Editing any setting in the visual modal validates input (rejecting unsafe `Ctrl+C` steals), updates `config.toml`, and hot-reloads immediately.
