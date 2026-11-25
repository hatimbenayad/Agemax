use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an interactive terminal surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SurfaceId(pub Uuid);

impl SurfaceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SurfaceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SurfaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Grid dimensions for a terminal surface in character columns and rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalSize {
    pub cols: u16,
    pub rows: u16,
}

impl TerminalSize {
    pub const DEFAULT: Self = Self { cols: 80, rows: 24 };

    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Cursor presentation styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CursorStyle {
    #[default]
    Block,
    Underline,
    Beam,
}

/// 2D point for layout and rendering coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// 2D rectangle for geometry, splits, and bounding boxes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Rect {
    pub min: Point,
    pub max: Point,
}

impl Rect {
    pub fn new(min: Point, max: Point) -> Self {
        Self { min, max }
    }

    pub fn from_min_size(min: Point, width: f32, height: f32) -> Self {
        Self {
            min,
            max: Point::new(min.x + width.max(0.0), min.y + height.max(0.0)),
        }
    }

    pub fn width(&self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    pub fn height(&self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
}

/// Forward-compatible pane surface kind (Unit 7 / Unit 11 stretch hooks).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SurfaceKind {
    #[default]
    Terminal,
    BrowserPreview {
        url: String,
    },
    AgentCardPanel {
        session_id: Option<String>,
    },
}

/// Metadata linking panes in a multi-agent team layout (Unit 7 §C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamRoleMeta {
    pub group_id: Uuid,
    pub role_name: String,
    pub is_coordinator: bool,
}
