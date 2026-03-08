use cosmic_text::{fontdb, Attrs, AttrsList, Family, FontSystem, Metrics, Shaping, Weight};

/// Verify that `optical_sizing(false)` produces different advance widths than the default
/// (`optical_sizing(true)`) for a variable font with an `opsz` axis.
///
/// Inter Variable has opsz range 14..32. At font_size=89.375, opsz is clamped to 32 (display),
/// which produces narrower glyphs than opsz=14 (body/default). Disabling optical sizing should
/// leave opsz at the font's default (14), yielding wider advances.
#[test]
fn optical_sizing_on_vs_off() {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    let font_data = std::fs::read("fonts/InterVariable.ttf").unwrap();
    font_system.db_mut().load_font_data(font_data);

    let text = "The Inter typeface family";
    let font_size = 89.375_f32;

    let attrs_on = Attrs::new()
        .family(Family::Name("Inter Variable"))
        .weight(Weight::BOLD);

    let attrs_off = Attrs::new()
        .family(Family::Name("Inter Variable"))
        .weight(Weight::BOLD)
        .optical_sizing(false);

    let line_on = cosmic_text::ShapeLine::new(
        &mut font_system,
        text,
        &AttrsList::new(&attrs_on),
        Shaping::Advanced,
        8,
        font_size,
    );

    let line_off = cosmic_text::ShapeLine::new(
        &mut font_system,
        text,
        &AttrsList::new(&attrs_off),
        Shaping::Advanced,
        8,
        font_size,
    );

    let total_advance = |line: &cosmic_text::ShapeLine| -> f32 {
        line.spans
            .iter()
            .flat_map(|s| &s.words)
            .flat_map(|w| &w.glyphs)
            .map(|g| g.x_advance)
            .sum::<f32>()
    };

    let advance_on = total_advance(&line_on);
    let advance_off = total_advance(&line_off);

    // opsz=32 (display) should be narrower than opsz=14 (body default)
    assert!(
        advance_on < advance_off,
        "Expected opsz=on (display, {advance_on:.4}) to be narrower than opsz=off (default, {advance_off:.4})"
    );

    // Inter shows ~5-6% difference between opsz=14 and opsz=32
    let pct_diff = (advance_off - advance_on) / advance_on * 100.0;
    assert!(
        pct_diff > 3.0,
        "Expected at least 3% advance width difference, got {pct_diff:.2}%"
    );
}

/// When per-span metrics override the font size, the opsz value used during
/// shaping must match the opsz that the renderer will use.
///
/// Shaping resolves `OpticalSize::Auto` from the buffer's base `font_size`
/// parameter, but the renderer resolves it from each `LayoutGlyph::font_size`
/// (which comes from per-span metrics). When these differ, the shaper computes
/// glyph advances at one opsz while the renderer rasterizes at another, causing
/// visible misalignment.
///
/// This test shapes identical text two ways:
///   1. buffer base=16, per-span font_size=89.375 (simulates rich text with
///      a size override — the common case in rich text widgets)
///   2. buffer base=89.375, no per-span override (consistent opsz)
///
/// Both produce glyphs at the same effective pixel size, so their EM-unit
/// advances must be equal. If shaping incorrectly resolves opsz from the buffer
/// base instead of the span's font size, case 1 shapes at opsz≈16 (clamped to
/// 14) while case 2 shapes at opsz≈89 (clamped to 32), producing different
/// advances.
#[test]
fn opsz_consistent_with_per_span_metrics() {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    let font_data = std::fs::read("fonts/InterVariable.ttf").unwrap();
    font_system.db_mut().load_font_data(font_data);

    let text = "The Inter typeface family";
    let buffer_base = 16.0_f32;
    let span_size = 89.375_f32;

    // Case 1: per-span metrics override font_size.
    // The buffer base is 16, but the span requests 89.375.
    // Shaping should use opsz appropriate for the *displayed* size (89.375),
    // not the buffer base (16).
    let attrs_with_metrics = Attrs::new()
        .family(Family::Name("Inter Variable"))
        .weight(Weight::BOLD)
        .metrics(Metrics::new(span_size, span_size * 1.2));

    let line_override = cosmic_text::ShapeLine::new(
        &mut font_system,
        text,
        &AttrsList::new(&attrs_with_metrics),
        Shaping::Advanced,
        8,
        buffer_base,
    );

    // Case 2: no per-span override — buffer base IS the display size.
    // Shaping and rendering both use opsz for 89.375.
    let attrs_direct = Attrs::new()
        .family(Family::Name("Inter Variable"))
        .weight(Weight::BOLD);

    let line_direct = cosmic_text::ShapeLine::new(
        &mut font_system,
        text,
        &AttrsList::new(&attrs_direct),
        Shaping::Advanced,
        8,
        span_size,
    );

    let total_advance = |line: &cosmic_text::ShapeLine| -> f32 {
        line.spans
            .iter()
            .flat_map(|s| &s.words)
            .flat_map(|w| &w.glyphs)
            .map(|g| g.x_advance)
            .sum::<f32>()
    };

    let advance_override = total_advance(&line_override);
    let advance_direct = total_advance(&line_direct);

    // Both should produce the same advances because the displayed font size
    // (and therefore the target opsz) is 89.375 in both cases.
    // A difference means shaping used the wrong opsz for case 1.
    let pct_diff = (advance_override - advance_direct).abs() / advance_direct * 100.0;
    assert!(
        pct_diff < 0.1,
        "opsz mismatch: per-span override advance ({advance_override:.4}) differs from \
         direct advance ({advance_direct:.4}) by {pct_diff:.2}% — shaping likely used \
         buffer base font_size for opsz instead of the span's font_size"
    );
}
