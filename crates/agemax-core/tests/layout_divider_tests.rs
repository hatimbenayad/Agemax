use agemax_core::layout::{LayoutTree, PaneId, SplitDirection};
use agemax_core::model::{Point, Rect};

#[test]
fn test_split_dividers_and_ratio_update() {
    let p1 = PaneId::new();
    let p2 = PaneId::new();
    let p3 = PaneId::new();

    let mut tree = LayoutTree::new_leaf(p1);
    assert!(tree.split(p1, SplitDirection::Horizontal, p2));
    assert!(tree.split(p2, SplitDirection::Vertical, p3));

    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), 1000.0, 600.0);
    let dividers = tree.split_dividers(bounds);
    assert_eq!(dividers.len(), 2);

    // Root divider runs vertically down the middle at x = 500
    let root_div = dividers.iter().find(|d| d.path.is_empty()).unwrap();
    assert_eq!(root_div.direction, SplitDirection::Horizontal);
    assert!((root_div.min.x - 500.0).abs() < 1.0);
    assert!((root_div.max.y - 600.0).abs() < 1.0);

    // Update root ratio with out-of-range check clamping to [0.10, 0.90]
    assert!(tree.set_split_ratio_at_path(&[], 0.98));
    let rects = tree.compute_rects(bounds);
    assert!((rects[&p1].width() - 900.0).abs() < 2.0);

    // Update child divider path [1]
    assert!(tree.set_split_ratio_at_path(&[1], 0.25));
    let updated_divs = tree.split_dividers(bounds);
    let child_div = updated_divs.iter().find(|d| d.path == vec![1]).unwrap();
    assert_eq!(child_div.direction, SplitDirection::Vertical);
    assert!((child_div.min.y - 150.0).abs() < 2.0);
}
