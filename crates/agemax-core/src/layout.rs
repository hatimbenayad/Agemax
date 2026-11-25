use crate::model::{Point, Rect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PaneId(pub Uuid);

impl PaneId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for PaneId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PaneId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TabId(pub Uuid);

impl TabId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TabId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub Uuid);

impl WorkspaceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WorkspaceId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    /// Splits left and right (vertical divider).
    Horizontal,
    /// Splits top and bottom (horizontal divider).
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Interactive split divider segment along with the tree path to the owning `LayoutTree::Split`.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitDivider {
    /// Path of child steps (`0` = first, `1` = second) from the root layout node to this split.
    pub path: Vec<u8>,
    pub direction: SplitDirection,
    pub min: Point,
    pub max: Point,
    pub container: Rect,
}

/// Binary tree representing pane splits inside a tab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayoutTree {
    Leaf(PaneId),
    Split {
        direction: SplitDirection,
        /// Split ratio between 0.05 and 0.95 (default 0.5).
        ratio: f32,
        first: Box<LayoutTree>,
        second: Box<LayoutTree>,
    },
}

impl LayoutTree {
    pub fn new_leaf(pane_id: PaneId) -> Self {
        Self::Leaf(pane_id)
    }

    /// Build a balanced split layout grid for multi-agent team orchestration (Unit 7 §C).
    pub fn create_team_grid(coordinator: PaneId, workers: &[PaneId]) -> Self {
        if workers.is_empty() {
            return Self::Leaf(coordinator);
        }

        fn balanced_workers(slice: &[PaneId], depth: usize) -> LayoutTree {
            match slice.len() {
                0 => unreachable!("non-empty slice checked by caller"),
                1 => LayoutTree::Leaf(slice[0]),
                _ => {
                    let mid = slice.len() / 2;
                    let dir = if depth.is_multiple_of(2) {
                        SplitDirection::Vertical
                    } else {
                        SplitDirection::Horizontal
                    };
                    LayoutTree::Split {
                        direction: dir,
                        ratio: 0.5,
                        first: Box::new(balanced_workers(&slice[..mid], depth + 1)),
                        second: Box::new(balanced_workers(&slice[mid..], depth + 1)),
                    }
                }
            }
        }

        Self::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.42,
            first: Box::new(Self::Leaf(coordinator)),
            second: Box::new(balanced_workers(workers, 0)),
        }
    }

    pub fn panes(&self) -> Vec<PaneId> {
        let mut list = Vec::new();
        self.collect_panes(&mut list);
        list
    }

    fn collect_panes(&self, list: &mut Vec<PaneId>) {
        match self {
            Self::Leaf(id) => list.push(*id),
            Self::Split { first, second, .. } => {
                first.collect_panes(list);
                second.collect_panes(list);
            }
        }
    }

    pub fn contains(&self, target: PaneId) -> bool {
        match self {
            Self::Leaf(id) => *id == target,
            Self::Split { first, second, .. } => first.contains(target) || second.contains(target),
        }
    }

    /// Split an existing pane in the specified direction.
    /// Returns true if the target pane was found and split.
    pub fn split(&mut self, target: PaneId, direction: SplitDirection, new_pane: PaneId) -> bool {
        match self {
            Self::Leaf(id) if *id == target => {
                let current_leaf = Self::Leaf(*id);
                let new_leaf = Self::Leaf(new_pane);
                *self = Self::Split {
                    direction,
                    ratio: 0.5,
                    first: Box::new(current_leaf),
                    second: Box::new(new_leaf),
                };
                true
            }
            Self::Leaf(_) => false,
            Self::Split { first, second, .. } => {
                first.split(target, direction, new_pane)
                    || second.split(target, direction, new_pane)
            }
        }
    }

    /// Remove a pane from the layout tree.
    /// Returns the updated tree, or None if the tree is now completely empty.
    pub fn remove(self, target: PaneId) -> Option<Self> {
        match self {
            Self::Leaf(id) if id == target => None,
            Self::Leaf(id) => Some(Self::Leaf(id)),
            Self::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let first_has = first.contains(target);
                let second_has = second.contains(target);

                if !first_has && !second_has {
                    return Some(Self::Split {
                        direction,
                        ratio,
                        first,
                        second,
                    });
                }

                if first_has {
                    match first.remove(target) {
                        Some(new_first) => Some(Self::Split {
                            direction,
                            ratio,
                            first: Box::new(new_first),
                            second,
                        }),
                        None => Some(*second), // Collapse to sibling
                    }
                } else {
                    match second.remove(target) {
                        Some(new_second) => Some(Self::Split {
                            direction,
                            ratio,
                            first,
                            second: Box::new(new_second),
                        }),
                        None => Some(*first), // Collapse to sibling
                    }
                }
            }
        }
    }

    /// Compute pixel rectangles for all leaves in the tree.
    pub fn compute_rects(&self, bounds: Rect) -> HashMap<PaneId, Rect> {
        let mut map = HashMap::new();
        self.collect_rects(bounds, &mut map);
        map
    }

    /// Compute interactive split dividers with exact binary paths for drag resizing.
    pub fn split_dividers(&self, bounds: Rect) -> Vec<SplitDivider> {
        let mut dividers = Vec::new();
        let mut path = Vec::new();
        self.collect_dividers(bounds, &mut path, &mut dividers);
        dividers
    }

    /// Update the ratio (`0.10..=0.90`) of the split node addressed by `path`.
    pub fn set_split_ratio_at_path(&mut self, path: &[u8], new_ratio: f32) -> bool {
        match (path.split_first(), self) {
            (None, Self::Split { ratio, .. }) => {
                *ratio = new_ratio.clamp(0.10, 0.90);
                true
            }
            (Some((&branch, rest)), Self::Split { first, second, .. }) => match branch {
                0 => first.set_split_ratio_at_path(rest, new_ratio),
                1 => second.set_split_ratio_at_path(rest, new_ratio),
                _ => false,
            },
            _ => false,
        }
    }

    fn collect_rects(&self, bounds: Rect, map: &mut HashMap<PaneId, Rect>) {
        match self {
            Self::Leaf(id) => {
                map.insert(*id, bounds);
            }
            Self::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (r1, r2, _) = partition_split(*direction, *ratio, bounds);
                first.collect_rects(r1, map);
                second.collect_rects(r2, map);
            }
        }
    }

    fn collect_dividers(&self, bounds: Rect, path: &mut Vec<u8>, out: &mut Vec<SplitDivider>) {
        if let Self::Split {
            direction,
            ratio,
            first,
            second,
        } = self
        {
            let (r1, r2, (min, max)) = partition_split(*direction, *ratio, bounds);
            out.push(SplitDivider {
                path: path.clone(),
                direction: *direction,
                min,
                max,
                container: bounds,
            });
            path.push(0);
            first.collect_dividers(r1, path, out);
            path.pop();
            path.push(1);
            second.collect_dividers(r2, path, out);
            path.pop();
        }
    }

    /// Find the adjacent pane in a given direction from a source pane.
    pub fn find_neighbor(&self, from: PaneId, dir: Direction, bounds: Rect) -> Option<PaneId> {
        let rects = self.compute_rects(bounds);
        let from_rect = rects.get(&from)?;
        let from_center = Point::new(
            from_rect.min.x + (from_rect.width() / 2.0),
            from_rect.min.y + (from_rect.height() / 2.0),
        );

        let mut best_candidate: Option<(PaneId, f32)> = None;

        for (candidate_id, candidate_rect) in &rects {
            if *candidate_id == from {
                continue;
            }
            let candidate_center = Point::new(
                candidate_rect.min.x + (candidate_rect.width() / 2.0),
                candidate_rect.min.y + (candidate_rect.height() / 2.0),
            );

            let is_in_direction = match dir {
                Direction::Left => candidate_center.x < from_center.x,
                Direction::Right => candidate_center.x > from_center.x,
                Direction::Up => candidate_center.y < from_center.y,
                Direction::Down => candidate_center.y > from_center.y,
            };

            if is_in_direction {
                let dx = candidate_center.x - from_center.x;
                let dy = candidate_center.y - from_center.y;
                let dist = (dx * dx) + (dy * dy);

                if let Some((_, best_dist)) = best_candidate {
                    if dist < best_dist {
                        best_candidate = Some((*candidate_id, dist));
                    }
                } else {
                    best_candidate = Some((*candidate_id, dist));
                }
            }
        }

        best_candidate.map(|(id, _)| id)
    }
}

fn partition_split(
    direction: SplitDirection,
    ratio: f32,
    bounds: Rect,
) -> (Rect, Rect, (Point, Point)) {
    let clamped_ratio = ratio.clamp(0.05, 0.95);
    match direction {
        SplitDirection::Horizontal => {
            let total_w = bounds.width();
            let w1 = (total_w * clamped_ratio).floor();
            let div_x = bounds.min.x + w1;
            let r1 = Rect::from_min_size(bounds.min, w1, bounds.height());
            let r2 = Rect::from_min_size(
                Point::new(div_x, bounds.min.y),
                total_w - w1,
                bounds.height(),
            );
            (
                r1,
                r2,
                (
                    Point::new(div_x, bounds.min.y),
                    Point::new(div_x, bounds.max.y),
                ),
            )
        }
        SplitDirection::Vertical => {
            let total_h = bounds.height();
            let h1 = (total_h * clamped_ratio).floor();
            let div_y = bounds.min.y + h1;
            let r1 = Rect::from_min_size(bounds.min, bounds.width(), h1);
            let r2 = Rect::from_min_size(
                Point::new(bounds.min.x, div_y),
                bounds.width(),
                total_h - h1,
            );
            (
                r1,
                r2,
                (
                    Point::new(bounds.min.x, div_y),
                    Point::new(bounds.max.x, div_y),
                ),
            )
        }
    }
}
