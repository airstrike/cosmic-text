use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SpanPadding};

/// Verify that `SpanPadding` on a middle span affects glyph positioning,
/// sets `padding_start`/`padding_end` on boundary glyphs, and increases
/// line height via max_ascent/max_descent.
#[test]
fn padding_offsets_and_line_height() {
    let mut font_system = FontSystem::new();
    let metrics = Metrics::new(16.0, 20.0);
    let attrs = Attrs::new();
    let padded = attrs.clone().padding(SpanPadding::uniform(10.0));

    // --- Padded layout ---
    let (padded_ascent, padded_descent, first_code_x, first_pad_start, last_pad_end) = {
        let mut buf = Buffer::new(&mut font_system, metrics);
        let mut buf = buf.borrow_with(&mut font_system);
        buf.set_rich_text(
            [
                ("Hello ", attrs.clone()),
                ("code", padded),
                (" world", attrs.clone()),
            ],
            &attrs,
            Shaping::Advanced,
            None,
        );
        buf.set_size(Some(800.0), Some(100.0));
        buf.shape_until_scroll(false);

        let run = buf
            .layout_runs()
            .next()
            .expect("should have at least one layout run");

        // Find glyphs belonging to the "code" span (bytes 6..10)
        let code_glyphs: Vec<_> = run
            .glyphs
            .iter()
            .filter(|g| g.start >= 6 && g.end <= 10)
            .collect();
        assert!(
            !code_glyphs.is_empty(),
            "should have glyphs for the 'code' span"
        );

        let first = code_glyphs.first().unwrap();
        let last = code_glyphs.last().unwrap();
        (
            run.max_ascent,
            run.max_descent,
            first.x,
            first.padding_start,
            last.padding_end,
        )
    };

    assert_eq!(
        first_pad_start, 10.0,
        "first glyph of padded span should have padding_start == 10.0"
    );
    assert_eq!(
        last_pad_end, 10.0,
        "last glyph of padded span should have padding_end == 10.0"
    );

    // --- Unpadded layout for comparison ---
    let (plain_ascent, plain_descent, plain_first_code_x) = {
        let mut buf = Buffer::new(&mut font_system, metrics);
        let mut buf = buf.borrow_with(&mut font_system);
        buf.set_text("Hello code world", &attrs, Shaping::Advanced, None);
        buf.set_size(Some(800.0), Some(100.0));
        buf.shape_until_scroll(false);

        let run = buf
            .layout_runs()
            .next()
            .expect("should have at least one layout run (plain)");

        let code_glyphs: Vec<_> = run
            .glyphs
            .iter()
            .filter(|g| g.start >= 6 && g.end <= 10)
            .collect();
        assert!(!code_glyphs.is_empty());

        (
            run.max_ascent,
            run.max_descent,
            code_glyphs.first().unwrap().x,
        )
    };

    // Line height should be larger with vertical padding
    let ascent_diff = padded_ascent - plain_ascent;
    let descent_diff = padded_descent - plain_descent;

    assert!(
        ascent_diff >= 9.9 && ascent_diff <= 10.1,
        "max_ascent should increase by ~10.0 from vertical padding, got diff={ascent_diff}"
    );
    assert!(
        descent_diff >= 9.9 && descent_diff <= 10.1,
        "max_descent should increase by ~10.0 from vertical padding, got diff={descent_diff}"
    );

    // First "code" glyph should be offset by padding_start
    let x_diff = first_code_x - plain_first_code_x;
    assert!(
        x_diff >= 9.9 && x_diff <= 10.1,
        "first 'code' glyph should be offset by ~10.0 from padding_start, got diff={x_diff}"
    );
}

/// Non-zero padding on boundary glyphs should be zero for unpadded spans.
#[test]
fn unpadded_glyphs_have_zero_padding() {
    let mut font_system = FontSystem::new();
    let metrics = Metrics::new(16.0, 20.0);
    let mut buffer = Buffer::new(&mut font_system, metrics);
    let mut buffer = buffer.borrow_with(&mut font_system);
    let attrs = Attrs::new();
    buffer.set_text("Hello world", &attrs, Shaping::Advanced, None);
    buffer.set_size(Some(800.0), Some(100.0));
    buffer.shape_until_scroll(false);

    let run = buffer
        .layout_runs()
        .next()
        .expect("should have a layout run");

    for glyph in run.glyphs {
        assert_eq!(
            glyph.padding_start, 0.0,
            "unpadded glyph at byte {} should have padding_start == 0.0",
            glyph.start
        );
        assert_eq!(
            glyph.padding_end, 0.0,
            "unpadded glyph at byte {} should have padding_end == 0.0",
            glyph.start
        );
    }
}
