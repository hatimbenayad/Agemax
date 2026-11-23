# Unit 3: Attention System, Visual Rings & Notifications

## 1. Scope
Implement the real-time agent attention system: OSC 7501 sequence parsing, terminal bell interception, high-contrast colored pane rings, sidebar unread badges, Windows taskbar flashing (`FlashWindowEx`), Windows Toast notifications (`windows::UI::Notifications`), attention panel, and global jump-to-unread shortcut.

## 2. Requirements Traceability
- **REQ-021**: Real-time OSC 7501 parser (`working`, `waiting`, `done`, `failed`).
- **REQ-022**: Terminal bell and secondary attention triggers (OSC 9/99/777).
- **REQ-023**: Colored visual rings around pane borders.
- **REQ-024**: Sidebar workspace attention badges with unread counters.
- **REQ-025**: Taskbar window flashing when unfocused.
- **REQ-026**: Native Windows Toast notification delivery.
- **REQ-027**: Clicking Windows Toast focuses multiplexer and targeting pane.
- **REQ-028**: Attention notification panel with chronological history.
- **REQ-029**: Global jump-to-unread shortcut (`Ctrl+Shift+U`).
- **REQ-030**: Automatic ring dismissal and badge decrement on user input.

## 3. Interfaces & Data Structures
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Working,
    WaitingForInput,
    Done,
    Failed,
}

pub struct NotificationItem {
    pub id: Uuid,
    pub pane_id: PaneId,
    pub workspace_id: WorkspaceId,
    pub status: AgentStatus,
    pub message: String,
    pub timestamp: std::time::SystemTime,
    pub read: bool,
}

pub trait NotificationService: Send + Sync {
    fn show_toast(&self, item: &NotificationItem) -> Result<(), NotificationError>;
    fn flash_taskbar(&self) -> Result<(), NotificationError>;
}
```

## 4. Acceptance Criteria
1. When an agent emits `\x1b]7501;status=waiting;prompt=approval\x07`, the pane border immediately renders an amber visual ring.
2. The sidebar workspace item shows a glowing unread badge and snippet ("waiting: approval").
3. If Age Max is minimized or unfocused, the Windows taskbar icon flashes, and a Windows Toast notification appears.
4. Clicking the toast brings Age Max to the foreground and focuses the waiting pane.
5. Pressing `Ctrl+Shift+U` jumps directly to the oldest unread waiting pane.
6. Typing any keystroke into the waiting pane dismisses the visual ring and marks the notification as read.

## 5. Test Plan
- Unit tests for OSC 7501 parser verifying BEL and ST terminators and parameter extraction.
- End-to-end integration test with a mock agent emitting status sequences.
- Unit test for `NotificationStore` unread queue and priority ordering.
- Manual verification of Windows Toast activation and taskbar flash on Windows 10/11.
