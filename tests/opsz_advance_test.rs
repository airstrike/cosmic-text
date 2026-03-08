use skrifa::instance::Size;
use skrifa::raw::types::Tag;
use skrifa::raw::ReadError;
use skrifa::{FontRef, GlyphId, MetadataProvider};

#[test]
fn test_opsz_affects_advance_width() -> Result<(), ReadError> {
    let data = std::fs::read("fonts/InterVariable.ttf").expect("Failed to read InterVariable.ttf");
    let font_ref = FontRef::from_index(&data, 0).unwrap();

    // Print available axes
    println!("\n=== Variable axes in InterVariable.ttf ===");
    for axis in font_ref.axes().iter() {
        println!(
            "  Tag: {:?}, min: {}, default: {}, max: {}",
            axis.tag(),
            axis.min_value(),
            axis.default_value(),
            axis.max_value()
        );
    }

    // Create locations at two different opsz values, both at wght=700
    let location_opsz14 = font_ref
        .axes()
        .location(&[(Tag::new(b"wght"), 700.0), (Tag::new(b"opsz"), 14.0)]);
    let location_opsz32 = font_ref
        .axes()
        .location(&[(Tag::new(b"wght"), 700.0), (Tag::new(b"opsz"), 32.0)]);

    let metrics_14 = font_ref.glyph_metrics(Size::unscaled(), &location_opsz14);
    let metrics_32 = font_ref.glyph_metrics(Size::unscaled(), &location_opsz32);

    let charmap = font_ref.charmap();

    let test_chars = ['T', 'H', 'a', 'e', 'i', 'W'];
    println!("\n=== Advance widths at wght=700, opsz=14 vs opsz=32 ===");
    let mut any_differ = false;
    for ch in test_chars {
        let gid = charmap.map(ch).unwrap();
        let adv_14 = metrics_14.advance_width(gid);
        let adv_32 = metrics_32.advance_width(gid);
        let differs = if adv_14 != adv_32 {
            " <-- DIFFERENT"
        } else {
            ""
        };
        if adv_14 != adv_32 {
            any_differ = true;
        }
        println!(
            "  '{}' (gid {:?}): opsz=14 -> {:?}, opsz=32 -> {:?}{}",
            ch, gid, adv_14, adv_32, differs
        );
    }

    println!("\n=== Also check at wght=400 ===");
    let location_400_14 = font_ref
        .axes()
        .location(&[(Tag::new(b"wght"), 400.0), (Tag::new(b"opsz"), 14.0)]);
    let location_400_32 = font_ref
        .axes()
        .location(&[(Tag::new(b"wght"), 400.0), (Tag::new(b"opsz"), 32.0)]);
    let metrics_400_14 = font_ref.glyph_metrics(Size::unscaled(), &location_400_14);
    let metrics_400_32 = font_ref.glyph_metrics(Size::unscaled(), &location_400_32);

    for ch in test_chars {
        let gid = charmap.map(ch).unwrap();
        let adv_14 = metrics_400_14.advance_width(gid);
        let adv_32 = metrics_400_32.advance_width(gid);
        let differs = if adv_14 != adv_32 {
            " <-- DIFFERENT"
        } else {
            ""
        };
        if adv_14 != adv_32 {
            any_differ = true;
        }
        println!(
            "  '{}' (gid {:?}): opsz=14 -> {:?}, opsz=32 -> {:?}{}",
            ch, gid, adv_14, adv_32, differs
        );
    }

    println!("\nAny differences found: {}", any_differ);

    Ok(())
}
