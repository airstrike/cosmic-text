use cosmic_text::{fontdb, Attrs, AttrsList, Family, FontSystem, Shaping, Weight};

/// Check whether vertical metrics (ascent, descent, line height) differ between opsz values.
#[test]
fn opsz_vertical_metrics() {
    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    let font_data = std::fs::read("fonts/InterVariable.ttf").unwrap();
    font_system.db_mut().load_font_data(font_data);

    let attrs = AttrsList::new(
        &Attrs::new()
            .family(Family::Name("Inter Variable"))
            .weight(Weight::BOLD),
    );

    let text = "The Inter typeface family";

    // Shape at font_size=14 → opsz=14
    let line_14 =
        cosmic_text::ShapeLine::new(&mut font_system, text, &attrs, Shaping::Advanced, 8, 14.0);

    // Shape at font_size=89.375 → opsz=32
    let line_89 =
        cosmic_text::ShapeLine::new(&mut font_system, text, &attrs, Shaping::Advanced, 8, 89.375);

    // Extract per-glyph ascent/descent from the first span
    let glyphs_14: Vec<_> = line_14.spans[0]
        .words
        .iter()
        .flat_map(|w| &w.glyphs)
        .collect();
    let glyphs_89: Vec<_> = line_89.spans[0]
        .words
        .iter()
        .flat_map(|w| &w.glyphs)
        .collect();

    eprintln!("=== Per-glyph vertical metrics ===");
    eprintln!(
        "opsz=14: ascent={:.6}, descent={:.6}",
        glyphs_14[0].ascent, glyphs_14[0].descent
    );
    eprintln!(
        "opsz=32: ascent={:.6}, descent={:.6}",
        glyphs_89[0].ascent, glyphs_89[0].descent
    );
    eprintln!(
        "ascent diff: {:.6}",
        glyphs_14[0].ascent - glyphs_89[0].ascent
    );
    eprintln!(
        "descent diff: {:.6}",
        glyphs_14[0].descent - glyphs_89[0].descent
    );

    // Also check via skrifa directly
    use skrifa::prelude::*;
    let font_bytes = std::fs::read("fonts/InterVariable.ttf").unwrap();
    let font_ref = skrifa::FontRef::from_index(&font_bytes, 0).unwrap();

    for opsz_val in [14.0_f32, 32.0] {
        let loc = font_ref.axes().location([
            (skrifa::Tag::new(b"wght"), 700.0),
            (skrifa::Tag::new(b"opsz"), opsz_val),
        ]);
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), &loc);
        eprintln!(
            "\nskrifa metrics at opsz={opsz_val}: ascent={}, descent={}, leading={}, units_per_em={}",
            metrics.ascent, metrics.descent, metrics.leading, metrics.units_per_em
        );
        eprintln!(
            "  cap_height={:?}, x_height={:?}",
            metrics.cap_height, metrics.x_height
        );
    }

    // Check MVAR table contents
    use skrifa::raw::TableProvider;
    let has_mvar = font_ref.mvar().is_ok();
    eprintln!("\nInter has MVAR table: {has_mvar}");

    if let Ok(mvar) = font_ref.mvar() {
        eprintln!("MVAR value_record_count: {}", mvar.value_record_count());
        for rec in mvar.value_records() {
            let tag = rec.value_tag();
            eprintln!(
                "  MVAR tag: {:?} (delta_set_outer={}, delta_set_inner={})",
                tag,
                rec.delta_set_outer_index(),
                rec.delta_set_inner_index()
            );
        }
    }

    // Check all skrifa metrics fields for differences
    for opsz_val in [14.0_f32, 32.0] {
        let loc = font_ref.axes().location([
            (skrifa::Tag::new(b"wght"), 700.0),
            (skrifa::Tag::new(b"opsz"), opsz_val),
        ]);
        let metrics = font_ref.metrics(skrifa::instance::Size::unscaled(), &loc);
        eprintln!("\nFull skrifa metrics at opsz={opsz_val}:");
        eprintln!("  units_per_em={}", metrics.units_per_em);
        eprintln!("  ascent={}", metrics.ascent);
        eprintln!("  descent={}", metrics.descent);
        eprintln!("  leading={}", metrics.leading);
        eprintln!("  average_width={:?}", metrics.average_width);
        eprintln!("  max_width={:?}", metrics.max_width);
        eprintln!("  cap_height={:?}", metrics.cap_height);
        eprintln!("  x_height={:?}", metrics.x_height);
        eprintln!("  full: {metrics:#?}");
    }
}
