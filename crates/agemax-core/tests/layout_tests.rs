use agemax_core::layout::{Direction, LayoutTree, PaneId, SplitDirection};
use agemax_core::model::{Point, Rect};
use proptest::prelude::*;

#[test]
fn test_single_leaf_layout() {
    let p1 = PaneId::new();
    let tree = LayoutTree::new_leaf(p1);

    assert_eq!(tree.panes(), vec![p1]);
    assert!(tree.contains(p1));

    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), 1000.0, 800.0);
    let rects = tree.compute_rects(bounds);

    assert_eq!(rects.len(), 1);
    assert_eq!(rects.get(&p1), Some(&bounds));
}

#[test]
fn test_horizontal_split() {
    let p1 = PaneId::new();
    let p2 = PaneId::new();
    let mut tree = LayoutTree::new_leaf(p1);

    let ok = tree.split(p1, SplitDirection::Horizontal, p2);
    assert!(ok);
    assert_eq!(tree.panes(), vec![p1, p2]);

    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), 1000.0, 800.0);
    let rects = tree.compute_rects(bounds);

    assert_eq!(rects.len(), 2);
    let r1 = rects.get(&p1).unwrap();
    let r2 = rects.get(&p2).unwrap();

    assert_eq!(r1.min.x, 0.0);
    assert_eq!(r1.width(), 500.0);
    assert_eq!(r2.min.x, 500.0);
    assert_eq!(r2.width(), 500.0);
}

#[test]
fn test_remove_leaf_collapses_to_sibling() {
    let p1 = PaneId::new();
    let p2 = PaneId::new();
    let mut tree = LayoutTree::new_leaf(p1);
    tree.split(p1, SplitDirection::Vertical, p2);

    let updated = tree.remove(p1).expect("Tree should not be empty");
    assert_eq!(updated, LayoutTree::Leaf(p2));
}

#[test]
fn test_directional_neighbor_navigation() {
    let p_left = PaneId::new();
    let p_right = PaneId::new();
    let mut tree = LayoutTree::new_leaf(p_left);
    tree.split(p_left, SplitDirection::Horizontal, p_right);

    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), 1000.0, 800.0);

    let neighbor_right = tree.find_neighbor(p_left, Direction::Right, bounds);
    assert_eq!(neighbor_right, Some(p_right));

    let neighbor_left = tree.find_neighbor(p_right, Direction::Left, bounds);
    assert_eq!(neighbor_left, Some(p_left));
}

proptest! {
    #[test]
    fn prop_rect_partition_preserves_area(ratio in 0.05f32..0.95f32) {
        let p1 = PaneId::new();
        let p2 = PaneId::new();
        let tree = LayoutTree::Split {
            direction: SplitDirection::Horizontal,
            ratio,
            first: Box::new(LayoutTree::Leaf(p1)),
            second: Box::new(LayoutTree::Leaf(p2)),
        };

        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), 800.0, 600.0);
        let rects = tree.compute_rects(bounds);

        assert_eq!(rects.len(), 2);
        let r1 = rects.get(&p1).unwrap();
        let r2 = rects.get(&p2).unwrap();

        let total_area = bounds.width() * bounds.height();
        let sub_area = (r1.width() * r1.height()) + (r2.width() * r2.height());

        // Area must match within pixel rounding tolerance
        let diff = (total_area - sub_area).abs();
        prop_assert!(diff < 5.0, "Area diff was {} (total: {}, sub: {})", diff, total_area, sub_area);
    }
}
