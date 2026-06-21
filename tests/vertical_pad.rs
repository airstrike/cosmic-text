use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, VerticalPad};

#[test]
fn vertical_pad_affects_scroll_extent() {
    let mut font_system = FontSystem::new();
    let metrics = Metrics::new(16.0, 20.0);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    buffer.set_text(
        "line1\nline2\nline3",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.set_size(Some(200.0), Some(100.0));

    // Without padding
    buffer.set_vertical_pad(VerticalPad {
        top: 0.0,
        bottom: 0.0,
    });
    buffer.shape_until_scroll(&mut font_system, false);
    let first_top_no_pad = buffer.layout_runs().next().map(|r| r.line_top);

    // With padding
    buffer.set_vertical_pad(VerticalPad {
        top: 10.0,
        bottom: 10.0,
    });
    buffer.shape_until_scroll(&mut font_system, false);
    let first_top_with_pad = buffer.layout_runs().next().map(|r| r.line_top);

    // The first run should be offset by pad_top
    if let (Some(no_pad), Some(with_pad)) = (first_top_no_pad, first_top_with_pad) {
        assert!(
            with_pad > no_pad,
            "pad_top should push the first run down: {} vs {}",
            with_pad,
            no_pad
        );
    }
}
