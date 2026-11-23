use agemax_core::model::{Point, Rect, SurfaceId, TerminalSize};

#[test]
fn test_surface_id_uniqueness() {
    let id1 = SurfaceId::new();
    let id2 = SurfaceId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_terminal_size_bounds() {
    let size = TerminalSize::new(0, 0);
    assert_eq!(size.cols, 1);
    assert_eq!(size.rows, 1);

    let custom = TerminalSize::new(120, 40);
    assert_eq!(custom.cols, 120);
    assert_eq!(custom.rows, 40);
}

#[test]
fn test_rect_geometry() {
    let r = Rect::from_min_size(Point::new(10.0, 20.0), 100.0, 50.0);
    assert_eq!(r.width(), 100.0);
    assert_eq!(r.height(), 50.0);
    assert!(r.contains(Point::new(15.0, 25.0)));
    assert!(!r.contains(Point::new(5.0, 25.0)));
    assert!(!r.contains(Point::new(150.0, 25.0)));
}
