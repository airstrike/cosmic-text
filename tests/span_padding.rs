use cosmic_text::{Attrs, Buffer, FontSystem, Metrics, Shaping, SpanPadding};

/// Verify that `SpanPadding` on a middle span affects glyph positioning,
/// sets `padding.start()`/`padding.end()` on boundary glyphs, and increases
/// line height via max_ascent/max_descent.
#[test]
fn padding_offsets_and_line_height() {
    let mut font_system = FontSystem::new();
    let metrics = Metrics::new(16.0, 20.0);
    let attrs = Attrs::new();
    let padded = attrs.clone().padding(SpanPadding::all(10.0));

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
            first.padding.start(),
            last.padding.end(),
        )
    };

    assert_eq!(
        first_pad_start, 10.0,
        "first glyph of padded span should have padding.start() == 10.0"
    );
    assert_eq!(
        last_pad_end, 10.0,
        "last glyph of padded span should have padding.end() == 10.0"
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

    // First "code" glyph should be offset by padding.start()
    let x_diff = first_code_x - plain_first_code_x;
    assert!(
        x_diff >= 9.9 && x_diff <= 10.1,
        "first 'code' glyph should be offset by ~10.0 from padding.start(), got diff={x_diff}"
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
            glyph.padding.start(),
            0.0,
            "unpadded glyph at byte {} should have padding.start() == 0.0",
            glyph.start
        );
        assert_eq!(
            glyph.padding.end(),
            0.0,
            "unpadded glyph at byte {} should have padding.end() == 0.0",
            glyph.start
        );
    }
}

fn line_height(
    font_system: &mut FontSystem,
    text: &str,
    attrs: &Attrs,
    padded_attrs: Option<(&str, &str, &Attrs)>,
) -> (f32, f32) {
    let metrics = Metrics::new(16.0, 20.0);
    let mut buf = Buffer::new(font_system, metrics);
    let mut buf = buf.borrow_with(font_system);
    if let Some((before, after, styled)) = padded_attrs {
        buf.set_rich_text(
            [
                (before, attrs.clone()),
                (text, styled.clone()),
                (after, attrs.clone()),
            ],
            attrs,
            Shaping::Advanced,
            None,
        );
    } else {
        buf.set_text(text, attrs, Shaping::Advanced, None);
    }
    buf.set_size(Some(800.0), Some(100.0));
    buf.shape_until_scroll(false);
    let run = buf.layout_runs().next().expect("should have a layout run");
    (run.max_ascent, run.max_descent)
}

#[test]
fn padded_spans_never_shrink_line_height() {
    let mut font_system = FontSystem::new();
    let plain = Attrs::new();

    let code_padded = Attrs::new()
        .family(cosmic_text::Family::Monospace)
        .padding(SpanPadding::new(1.0, 1.0, 6.0, 6.0));

    let formula_padded = Attrs::new().padding(SpanPadding::new(4.0, 4.0, 3.0, 3.0));

    // Baseline: plain text only
    let (plain_ascent, plain_descent) = line_height(&mut font_system, "Hello world", &plain, None);
    let plain_total = plain_ascent + plain_descent;
    assert!(
        plain_total > 0.0,
        "plain text should have positive line height"
    );

    // Code span only (monospace + 1px vertical padding)
    let (code_ascent, code_descent) = line_height(
        &mut font_system,
        "abcde",
        &plain,
        Some(("", "", &code_padded)),
    );
    let code_total = code_ascent + code_descent;
    assert!(
        code_total >= plain_total,
        "code span line height ({code_total}) must be >= plain ({plain_total})"
    );

    // Formula span only (4px vertical padding)
    let (form_ascent, form_descent) = line_height(
        &mut font_system,
        "3",
        &plain,
        Some(("", "", &formula_padded)),
    );
    let form_total = form_ascent + form_descent;
    assert!(
        form_total >= plain_total,
        "formula span line height ({form_total}) must be >= plain ({plain_total})"
    );

    // Both: code + formula on same line
    let metrics = Metrics::new(16.0, 20.0);
    let mut buf = Buffer::new(&mut font_system, metrics);
    let mut buf = buf.borrow_with(&mut font_system);
    buf.set_rich_text(
        [
            ("Hello ", plain.clone()),
            ("abcde", code_padded.clone()),
            (" and ", plain.clone()),
            ("3", formula_padded.clone()),
            (" end", plain.clone()),
        ],
        &plain,
        Shaping::Advanced,
        None,
    );
    buf.set_size(Some(800.0), Some(100.0));
    buf.shape_until_scroll(false);
    let run = buf.layout_runs().next().expect("layout run");
    let both_total = run.max_ascent + run.max_descent;
    assert!(
        both_total >= plain_total,
        "mixed line height ({both_total}) must be >= plain ({plain_total})"
    );
    assert!(
        both_total >= form_total,
        "mixed line height ({both_total}) must be >= formula-only ({form_total})"
    );

    // Code span with SMALLER font size (14px in a 16px buffer)
    let small_code = Attrs::new()
        .family(cosmic_text::Family::Monospace)
        .metrics(Metrics::new(14.0, 14.0))
        .padding(SpanPadding::new(1.0, 1.0, 6.0, 6.0));

    let (small_ascent, small_descent) = line_height(
        &mut font_system,
        "abcde",
        &plain,
        Some(("", "", &small_code)),
    );
    let small_total = small_ascent + small_descent;

    // Also check with NO padding at 14px to see raw shrinkage
    let small_nopad = Attrs::new()
        .family(cosmic_text::Family::Monospace)
        .metrics(Metrics::new(14.0, 14.0));
    let (nopad_ascent, nopad_descent) = line_height(
        &mut font_system,
        "abcde",
        &plain,
        Some(("", "", &small_nopad)),
    );
    let nopad_total = nopad_ascent + nopad_descent;

    eprintln!("plain:       asc={plain_ascent:.2} desc={plain_descent:.2} total={plain_total:.2}");
    eprintln!("code(14+0):  asc={nopad_ascent:.2} desc={nopad_descent:.2} total={nopad_total:.2}");
    eprintln!("code(14+1):  asc={small_ascent:.2} desc={small_descent:.2} total={small_total:.2}");
    eprintln!("formula(+4): asc={form_ascent:.2} desc={form_descent:.2} total={form_total:.2}");

    assert!(
        small_total >= plain_total,
        "small-font code span ({small_total}) must be >= plain ({plain_total}); \
         smaller font + padding must not shrink line height"
    );

    // Simulate iced's line_height_ratio: size 14 with ratio 1.6
    // → Metrics::new(14.0, 22.4) vs default Metrics::new(16.0, 25.6).
    // The line_height_opt from the smaller font must not shrink the line.
    let iced_code = Attrs::new()
        .family(cosmic_text::Family::Monospace)
        .metrics(Metrics::new(14.0, 14.0 * 1.6))
        .padding(SpanPadding::new(1.0, 1.0, 6.0, 6.0));
    let iced_metrics = Metrics::new(16.0, 16.0 * 1.6);

    let mut buf = Buffer::new(&mut font_system, iced_metrics);
    let mut buf = buf.borrow_with(&mut font_system);
    buf.set_text("Hello world", &plain, Shaping::Advanced, None);
    buf.set_size(Some(800.0), Some(100.0));
    buf.shape_until_scroll(false);
    let run = buf.layout_runs().next().unwrap();
    let iced_plain_height = run.line_height;
    eprintln!("iced plain: line_height={iced_plain_height:.2}");
    drop(buf);

    let mut buf = Buffer::new(&mut font_system, iced_metrics);
    let mut buf = buf.borrow_with(&mut font_system);
    buf.set_rich_text(
        [
            ("", plain.clone()),
            ("abcde", iced_code),
            ("", plain.clone()),
        ],
        &plain,
        Shaping::Advanced,
        None,
    );
    buf.set_size(Some(800.0), Some(100.0));
    buf.shape_until_scroll(false);
    let run = buf.layout_runs().next().unwrap();
    let iced_code_height = run.line_height;
    eprintln!("iced code:  line_height={iced_code_height:.2}");

    assert!(
        iced_code_height >= iced_plain_height,
        "code-only line height ({iced_code_height:.2}) must be >= plain ({iced_plain_height:.2}); \
         smaller font size in a span must not shrink the line"
    );
}
