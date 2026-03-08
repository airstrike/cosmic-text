use cosmic_text::{fontdb, Attrs, AttrsList, Family, FontSystem, ShapeLine, Shaping, Weight};

/// Verify that different font sizes produce different shaping advance widths
/// due to the `opsz` (optical size) variable font axis.
#[test]
fn opsz_affects_shaping_advance_widths() {
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

    // Shape at font_size=14 -> opsz axis = 14 (Inter's default/min)
    let line_14 = ShapeLine::new(&mut font_system, text, &attrs, Shaping::Advanced, 8, 14.0);

    // Shape at font_size=89 -> opsz axis = 32 (Inter's max, clamped from 89)
    let line_89 = ShapeLine::new(&mut font_system, text, &attrs, Shaping::Advanced, 8, 89.0);

    // Collect advance widths from both
    let advances_14: Vec<f32> = line_14.spans[0]
        .words
        .iter()
        .flat_map(|w| w.glyphs.iter().map(|g| g.x_advance))
        .collect();
    let advances_89: Vec<f32> = line_89.spans[0]
        .words
        .iter()
        .flat_map(|w| w.glyphs.iter().map(|g| g.x_advance))
        .collect();

    println!("Advances at font_size=14 (opsz=14): {:?}", advances_14);
    println!("Advances at font_size=89 (opsz=32): {:?}", advances_89);

    // They must have the same number of glyphs
    assert_eq!(
        advances_14.len(),
        advances_89.len(),
        "Different number of glyphs"
    );

    // At least some advance widths must differ (Inter's opsz changes metrics)
    let any_different = advances_14
        .iter()
        .zip(advances_89.iter())
        .any(|(a, b)| (a - b).abs() > 1e-6);

    assert!(
        any_different,
        "All advance widths are identical — opsz is NOT affecting shaping!\n\
         opsz=14: {:?}\n\
         opsz=32: {:?}",
        advances_14, advances_89
    );

    // Compute total line widths
    let total_14: f32 = advances_14.iter().sum();
    let total_89: f32 = advances_89.iter().sum();
    let pct_diff = ((total_14 - total_89) / total_14 * 100.0).abs();
    println!(
        "Total advance: opsz=14: {:.4}, opsz=32: {:.4}, diff: {:.2}%",
        total_14, total_89, pct_diff
    );

    // Inter's opsz=14 glyphs are wider than opsz=32 (body vs display)
    // We expect a measurable difference (empirically ~3-5%)
    assert!(
        pct_diff > 1.0,
        "opsz difference too small: {:.2}% — expected > 1%",
        pct_diff
    );
}
