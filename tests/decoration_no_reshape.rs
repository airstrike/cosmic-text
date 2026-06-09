//! Approach A: a decoration-only change must not reshape the line, while a
//! shaping-relevant change (font size, weight) still does.
//!
//! `BufferLine::set_attrs_list` returns `true` iff it reset shaping, which is
//! the signal these tests assert on. They also confirm the shaped glyph
//! identities are unchanged across a decoration toggle.

use cosmic_text::{
    Attrs, AttrsList, AttrsOverride, Buffer, Color, FontSystem, Metrics, Override, Shaping,
    TextDecoration, UnderlineStyle,
};

fn underline_over() -> AttrsOverride {
    AttrsOverride {
        text_decoration: Override::Set(TextDecoration {
            underline: UnderlineStyle::Single,
            ..TextDecoration::new()
        }),
        ..Default::default()
    }
}

fn shaped_buffer(fs: &mut FontSystem) -> Buffer {
    let mut buffer = Buffer::new(fs, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(400.0), None);
    buffer.set_text(
        "hello underlined world",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fs, false);
    buffer
}

/// Snapshot the shaped glyph identities of line 0 (font + glyph id + advance).
fn glyph_identities(buffer: &Buffer) -> Vec<(cosmic_text::fontdb::ID, u16, f32)> {
    buffer
        .layout_runs()
        .flat_map(|run| {
            run.glyphs
                .iter()
                .map(|g| (g.font_id, g.glyph_id, g.w))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn decoration_only_change_does_not_reshape() {
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);

    let before = glyph_identities(&buffer);

    // Add an underline span — a render-time-only change.
    let base = Attrs::new();
    let mut list = AttrsList::new(&base);
    list.add_span(6..16, &underline_over());

    let reshaped = buffer.lines[0].set_attrs_list(list);
    assert!(!reshaped, "a decoration-only change must not reshape");

    // Re-shape (a no-op if nothing was invalidated) and confirm glyphs match.
    buffer.shape_until_scroll(&mut fs, false);
    let after = glyph_identities(&buffer);
    assert_eq!(
        before, after,
        "shaped glyphs changed after a decoration toggle"
    );
}

#[test]
fn removing_decoration_does_not_reshape() {
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);

    let base = Attrs::new();
    let mut list = AttrsList::new(&base);
    list.add_span(6..16, &underline_over());
    assert!(!buffer.lines[0].set_attrs_list(list));
    buffer.shape_until_scroll(&mut fs, false);

    // Now clear it back to plain — also render-time only.
    let plain = AttrsList::new(&base);
    let reshaped = buffer.lines[0].set_attrs_list(plain);
    assert!(!reshaped, "removing a decoration must not reshape");
}

#[test]
fn font_size_change_reshapes() {
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);

    // A line-defaults metrics change is shaping-relevant.
    let bigger = Attrs::new().metrics(Metrics::new(28.0, 34.0));
    let list = AttrsList::new(&bigger);
    let reshaped = buffer.lines[0].set_attrs_list(list);
    assert!(reshaped, "a font-size change must reshape (control)");
}

#[test]
fn weight_change_reshapes() {
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);

    let base = Attrs::new();
    let mut list = AttrsList::new(&base);
    list.add_span(
        6..16,
        &AttrsOverride {
            weight: Override::Set(cosmic_text::Weight::BOLD),
            ..Default::default()
        },
    );
    let reshaped = buffer.lines[0].set_attrs_list(list);
    assert!(reshaped, "a weight change must reshape (control)");
}

#[test]
fn font_decoration_metrics_are_reachable_without_reshaping() {
    // The render-time resolver must be able to obtain per-font decoration
    // metrics from a glyph's font_id without touching shaping.
    let mut fs = FontSystem::new();
    let buffer = shaped_buffer(&mut fs);

    let font_id = buffer
        .layout_runs()
        .next()
        .and_then(|run| run.glyphs.first())
        .map(|g| g.font_id)
        .expect("expected at least one shaped glyph");

    let m = fs
        .decoration_metrics(font_id)
        .expect("font should be loaded");
    // Underline has positive thickness; ascent is positive.
    assert!(m.underline.thickness > 0.0);
    assert!(m.ascent > 0.0);
}

#[test]
fn color_change_still_reshapes_for_now() {
    // Color is still baked at shape time; until live color resolution lands a
    // color change must reshape so the new color reaches the glyphs.
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);

    let base = Attrs::new();
    let mut list = AttrsList::new(&base);
    list.add_span(
        6..16,
        &AttrsOverride {
            color: Override::Set(Some(Color::rgb(0x42, 0x85, 0xf4))),
            ..Default::default()
        },
    );
    let reshaped = buffer.lines[0].set_attrs_list(list);
    assert!(
        reshaped,
        "a color change must still reshape (pre-live-color)"
    );
}
