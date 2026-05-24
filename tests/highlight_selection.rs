//! Regression test: `LayoutRun::highlight` must not bleed selection
//! across lines outside the cursor range.
//!
//! Pre-existing bug (introduced by commit aa2c305039 "fix: selection
//! and highlight for mixed bidi text", March 2026): the per-glyph
//! `is_selected` check uses OR-short-circuits on the line index,
//! which makes every glyph on a line outside `[start.line, end.line]`
//! erroneously report as selected.

use cosmic_text::{Attrs, Buffer, Cursor, FontSystem, Metrics, Shaping};

#[test]
fn highlight_does_not_bleed_across_lines() {
    let mut font_system = FontSystem::new();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 18.0));
    buffer.set_size(Some(500.0), None);
    buffer.set_text(
        "first line\nsecond line\nthird line",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(&mut font_system, false);

    // Selection (0,0) → (0,1) — just the first character.
    let start = Cursor::new(0, 0);
    let end = Cursor::new(0, 1);

    for run in buffer.layout_runs() {
        let regions: Vec<_> = run.highlight(start, end).collect();
        if run.line_i == 0 {
            assert!(
                !regions.is_empty(),
                "line 0 should have at least one highlight region for the selected character",
            );
        } else {
            assert!(
                regions.is_empty(),
                "selection bled to line {}: returned {} region(s): {regions:?}",
                run.line_i,
                regions.len(),
            );
        }
    }
}

#[test]
fn highlight_spans_middle_lines_in_multi_line_selection() {
    // Sanity: when the selection genuinely crosses lines, the in-between
    // lines should be fully selected.
    let mut font_system = FontSystem::new();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 18.0));
    buffer.set_size(Some(500.0), None);
    buffer.set_text(
        "first line\nsecond line\nthird line",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(&mut font_system, false);

    // (0,5) → (2,5) — middle line should be fully selected.
    let start = Cursor::new(0, 5);
    let end = Cursor::new(2, 5);

    for run in buffer.layout_runs() {
        let regions: Vec<_> = run.highlight(start, end).collect();
        assert!(
            !regions.is_empty(),
            "line {} should have at least one highlight region",
            run.line_i,
        );
    }
}
