# Unit 9: Everyday Mouse Ergonomics, Live Theming & Startup Session Restore

## 1. Scope
Deliver intuitive mouse interactions, live visual customization from TOML themes, and seamless session continuation on relaunch:
1. Mouse wheel scrollback history in terminal surfaces with a subtle scrollbar indicator and "Jump to bottom" button.
2. Click-and-drag text selection, `Ctrl+Shift+C` copy, right-click context menu (*Copy*, *Paste*, *Clear*, *Split Right*, *Split Down*), and `Ctrl+Click` URL / OSC 8 hyperlink opening.
3. Drag-to-resize split dividers (`REQ-016`) and live font zoom (`Ctrl++`, `Ctrl+-`, `Ctrl+0`, `Ctrl+Wheel`).
4. Live `ThemeConfig` and `GeneralConfig.font_size` rendering in `agemax-app` (`REQ-057`) plus built-in presets (*Midnight Dark*, *High Contrast Dark*, *Soft Light*).
5. Startup `session.json` restore prompt (`REQ-049`, `REQ-051`) and inline pane-exit banner (`REQ-020`: `[Restart]`, `[Resume Agent]`, `[Close]`).

## 2. Requirements Traceability
- **REQ-005**, **REQ-006**, **REQ-007**, **REQ-016**, **REQ-020**, **REQ-049**, **REQ-051**, **REQ-057**, **REQ-070**–**REQ-074**.

## 3. Acceptance Criteria
1. Scrolling mouse wheel navigates scrollback lines cleanly; typing snaps back to bottom.
2. Selecting text with the mouse copies accurate multi-line text without trailing blanks via `Ctrl+Shift+C` or right-click **Copy**.
3. Dragging splitter dividers updates `LayoutNode::Split { ratio }` smoothly within `[0.1, 0.9]`.
4. Changing `theme` or `font_size` in `config.toml` updates colors and glyph sizing live within one frame.
5. Launching with an existing `session.json` offers a non-blocking restore bar with per-pane `agy --conversation <id>` resume buttons.
