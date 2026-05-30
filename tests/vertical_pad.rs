use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Scroll, Shaping, VerticalPad};

// `VerticalPad` reserves space inside the scroll extent rather than shrinking
// the viewport: `top` offsets the first line at scroll origin, and `bottom`
// is held below the last line at maximum scroll.

#[test]
fn top_pad_offsets_first_line() {
    let mut font_system = FontSystem::new();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, 20.0));
    let mut buffer = buffer.borrow_with(&mut font_system);

    buffer.set_text(
        "Line 0\nLine 1\nLine 2",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.set_size(Some(500.0), Some(500.0));

    let unpadded = buffer.layout_runs().next().expect("a layout run").line_top;
    assert_eq!(unpadded, 0.0);

    buffer.set_vertical_pad(VerticalPad {
        top: 10.0,
        bottom: 0.0,
    });

    let padded = buffer.layout_runs().next().expect("a layout run").line_top;
    assert_eq!(padded, 10.0);
}

#[test]
fn bottom_pad_held_below_last_line_at_max_scroll() {
    let line_height = 20.0;
    let viewport = 100.0;
    let pad_bottom = 15.0;

    let mut font_system = FontSystem::new();
    let mut buffer = Buffer::new(&mut font_system, Metrics::new(14.0, line_height));
    let mut buffer = buffer.borrow_with(&mut font_system);

    // Ten 20px lines (200px) in a 100px viewport: content exceeds the viewport.
    let text = (0..10)
        .map(|i| format!("Line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    buffer.set_text(&text, &Attrs::new(), Shaping::Advanced, None);
    buffer.set_size(Some(500.0), Some(viewport));
    buffer.set_vertical_pad(VerticalPad {
        top: 0.0,
        bottom: pad_bottom,
    });

    // Scroll past the end; shape_until_scroll clamps to the true maximum.
    buffer.set_scroll(Scroll::new(0, 10_000.0, 0.0));

    let last = buffer.layout_runs().last().expect("a layout run");
    let last_bottom = last.line_top + last.line_height;
    assert!(
        (last_bottom - (viewport - pad_bottom)).abs() < 1.0,
        "last line bottom {last_bottom}, expected {}",
        viewport - pad_bottom
    );
}
