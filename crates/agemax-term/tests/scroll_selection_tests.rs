use agemax_term::vt::{GridPoint, TerminalGrid, TerminalGridConfig};

#[test]
fn test_scrollback_delta_and_selection_extraction() {
    let mut grid = TerminalGrid::new(TerminalGridConfig { cols: 40, rows: 10 });
    for i in 1..=25 {
        grid.advance_bytes(format!("Line {:02} payload\r\n", i).as_bytes());
    }

    assert!(grid.history_size() > 0);
    assert_eq!(grid.display_offset(), 0);

    grid.scroll_delta(5);
    assert_eq!(grid.display_offset(), 5);

    // Viewport row 0 now points 5 lines into scrollback history
    let top_visible = grid.render_viewport_line_text(0);
    assert!(top_visible.starts_with("Line "), "Got: {:?}", top_visible);

    let start = GridPoint {
        line: grid.viewport_to_grid_line(0),
        col: 0,
    };
    let end = GridPoint {
        line: grid.viewport_to_grid_line(1),
        col: 14,
    };
    let extracted = grid.extract_selection_text(start, end);
    assert!(extracted.contains('\n'));
    assert!(extracted.starts_with("Line "));

    grid.scroll_to_bottom();
    assert_eq!(grid.display_offset(), 0);
}

#[test]
fn test_url_at_detects_plain_http_and_osc8_hyperlinks() {
    let mut grid = TerminalGrid::new(TerminalGridConfig { cols: 80, rows: 10 });
    grid.advance_bytes(b"Visit https://github.com/hatimbenayad/Agemax now\r\n");
    grid.advance_bytes(b"\x1b]8;;https://antigravity.google\x07Click Docs\x1b]8;;\x07 done\r\n");

    // Plain URL in row 0
    assert_eq!(
        grid.url_at(0, 10).as_deref(),
        Some("https://github.com/hatimbenayad/Agemax")
    );
    assert_eq!(grid.url_at(0, 0), None);
    assert_eq!(grid.url_at(0, 47), None);

    // OSC 8 hyperlink in row 1
    assert_eq!(
        grid.url_at(1, 2).as_deref(),
        Some("https://antigravity.google")
    );
    assert_eq!(grid.url_at(1, 13), None);
}
