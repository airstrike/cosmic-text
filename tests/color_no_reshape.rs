//! A color-only change must not reshape the line, and the new color must be
//! present after the relayout, since color is resolved at layout rather than
//! captured at shape time.

use cosmic_text::{Attrs, AttrsList, Buffer, Color, FontSystem, Metrics, Shaping};

fn glyph_ids(buffer: &Buffer) -> Vec<(cosmic_text::fontdb::ID, u16, f32)> {
    buffer
        .layout_runs()
        .flat_map(|run| {
            run.glyphs
                .iter()
                .map(|glyph| (glyph.font_id, glyph.glyph_id, glyph.w))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn span_colors(buffer: &Buffer, range: std::ops::Range<usize>) -> Vec<Option<Color>> {
    buffer
        .layout_runs()
        .flat_map(|run| {
            run.glyphs
                .iter()
                .filter(|glyph| range.contains(&glyph.start))
                .map(|glyph| glyph.color_opt)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn shaped_buffer(fs: &mut FontSystem) -> Buffer {
    let mut buffer = Buffer::new(fs, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(400.0), None);
    buffer.set_text(
        "hello colorful world",
        &Attrs::new(),
        Shaping::Advanced,
        None,
    );
    buffer.shape_until_scroll(fs, false);
    buffer
}

#[test]
fn color_only_change_does_not_reshape_and_recolors() {
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);
    let before = glyph_ids(&buffer);
    assert!(!before.is_empty());
    assert!(span_colors(&buffer, 6..14).iter().all(Option::is_none));

    let red = Color::rgb(0xFF, 0x00, 0x00);
    let mut colored = Attrs::new();
    colored.color_opt = Some(red);
    let mut list = AttrsList::new(&Attrs::new());
    list.add_span(6..14, &colored);

    let reshaped = buffer.lines[0].set_attrs_list(list);
    assert!(!reshaped, "a color-only change must not reshape");

    buffer.shape_until_scroll(&mut fs, false);
    assert_eq!(
        before,
        glyph_ids(&buffer),
        "shaped glyphs changed on a color-only change"
    );

    // The relayout must serve the new color, not a stale shape-time capture.
    let colors = span_colors(&buffer, 6..14);
    assert!(!colors.is_empty());
    assert!(
        colors.iter().all(|color| *color == Some(red)),
        "stale color after a color-only relayout"
    );

    // Reverting to the defaults clears it again, also without reshaping.
    let reshaped = buffer.lines[0].set_attrs_list(AttrsList::new(&Attrs::new()));
    assert!(!reshaped);
    buffer.shape_until_scroll(&mut fs, false);
    assert!(span_colors(&buffer, 6..14).iter().all(Option::is_none));
}

#[test]
fn color_with_weight_change_reshapes() {
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);

    let mut bold_red = Attrs::new();
    bold_red.color_opt = Some(Color::rgb(0xFF, 0x00, 0x00));
    bold_red.weight = cosmic_text::Weight::BOLD;
    let mut list = AttrsList::new(&Attrs::new());
    list.add_span(0..5, &bold_red);

    let reshaped = buffer.lines[0].set_attrs_list(list);
    assert!(reshaped, "a weight change must reshape (control)");
}

#[test]
fn multiple_color_spans_resolve_per_run() {
    let mut fs = FontSystem::new();
    let mut buffer = shaped_buffer(&mut fs);

    // "hello colorful world": red over "hello", default gap, blue over "world".
    let red = Color::rgb(0xFF, 0x00, 0x00);
    let blue = Color::rgb(0x00, 0x00, 0xFF);
    let mut red_attrs = Attrs::new();
    red_attrs.color_opt = Some(red);
    let mut blue_attrs = Attrs::new();
    blue_attrs.color_opt = Some(blue);
    let mut list = AttrsList::new(&Attrs::new());
    list.add_span(0..5, &red_attrs);
    list.add_span(15..20, &blue_attrs);

    let reshaped = buffer.lines[0].set_attrs_list(list);
    assert!(!reshaped);
    buffer.shape_until_scroll(&mut fs, false);

    assert!(span_colors(&buffer, 0..5)
        .iter()
        .all(|color| *color == Some(red)));
    assert!(span_colors(&buffer, 5..15).iter().all(Option::is_none));
    assert!(span_colors(&buffer, 15..20)
        .iter()
        .all(|color| *color == Some(blue)));
}
