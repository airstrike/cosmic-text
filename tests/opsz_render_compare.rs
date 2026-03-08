use cosmic_text::{
    fontdb, Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, SwashCache, Weight, Wrap,
};
use tiny_skia::{Paint, Pixmap, PremultipliedColorU8, Rect, Transform};

/// Render "The Inter\ntypeface family" at 89.375px Bold with Inter Variable,
/// producing two PNGs — one with opsz active and one with opsz disabled —
/// so the user can overlay them like the Chrome demo.
///
/// Run with:  cargo test opsz_render_compare -- --nocapture
///
/// Outputs:   tests/images/opsz_on.png    (optical_sizing=auto → opsz clamped to 32)
///            tests/images/opsz_off.png   (optical_sizing=false → opsz stays at font default 14)
///            tests/images/opsz_overlay.png (red/green diff overlay)
#[test]
fn opsz_render_compare() {
    let repo_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut font_system =
        FontSystem::new_with_locale_and_db("en-US".into(), fontdb::Database::new());
    let font_data = std::fs::read("fonts/InterVariable.ttf").unwrap();
    font_system.db_mut().load_font_data(font_data);

    let text = "The Inter\ntypeface family";
    let font_size = 89.375_f32;
    let line_height = font_size * 1.2;
    let canvas_w = 700_u32;
    let canvas_h = 240_u32;

    let base_attrs = Attrs::new()
        .family(Family::Name("Inter Variable"))
        .weight(Weight::BOLD)
        .letter_spacing(-4.0 / font_size);

    // --- opsz ON (auto: font_size=89.375 → opsz clamped to 32) ---
    let pixmap_on = {
        let attrs = base_attrs.clone();
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        {
            let mut b = buffer.borrow_with(&mut font_system);
            b.set_wrap(Wrap::None);
            b.set_size(Some(canvas_w as f32), Some(canvas_h as f32));
            b.set_text(text, &attrs, Shaping::Advanced, None);
            b.shape_until_scroll(false);
        }

        for run in buffer.layout_runs() {
            eprintln!("opsz=on  | line_w={:.2}px text={:?}", run.line_w, run.text);
        }

        let pixmap = render_buffer(&mut buffer, &mut font_system, canvas_w, canvas_h);
        let path = format!("{repo_dir}/tests/images/opsz_on.png");
        pixmap.save_png(&path).unwrap();
        eprintln!("Saved {path}");
        pixmap
    };

    // --- opsz OFF (optical_sizing=false → font default opsz=14) ---
    let pixmap_off = {
        let attrs = base_attrs.optical_sizing(false);
        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(&mut font_system, metrics);
        {
            let mut b = buffer.borrow_with(&mut font_system);
            b.set_wrap(Wrap::None);
            b.set_size(Some(canvas_w as f32), Some(canvas_h as f32));
            b.set_text(text, &attrs, Shaping::Advanced, None);
            b.shape_until_scroll(false);
        }

        for run in buffer.layout_runs() {
            eprintln!("opsz=off | line_w={:.2}px text={:?}", run.line_w, run.text);
        }

        let pixmap = render_buffer(&mut buffer, &mut font_system, canvas_w, canvas_h);
        let path = format!("{repo_dir}/tests/images/opsz_off.png");
        pixmap.save_png(&path).unwrap();
        eprintln!("Saved {path}");
        pixmap
    };

    // --- Red/green overlay (like Chrome's opsz demo diff) ---
    {
        let mut overlay = Pixmap::new(canvas_w, canvas_h).unwrap();
        overlay.fill(tiny_skia::Color::BLACK);

        for y in 0..canvas_h {
            for x in 0..canvas_w {
                let idx = (y * canvas_w + x) as usize;
                let on_pixel = pixmap_on.pixels()[idx];
                let off_pixel = pixmap_off.pixels()[idx];
                // Black text on white: darker = more text coverage
                let on_intensity = 255 - on_pixel.red();
                let off_intensity = 255 - off_pixel.red();
                // Red = opsz OFF (body/wider), Green = opsz ON (display/narrower)
                let r = off_intensity;
                let g = on_intensity;
                if r > 10 || g > 10 {
                    *overlay.pixels_mut().get_mut(idx).unwrap() =
                        PremultipliedColorU8::from_rgba(r, g, 0, 255).unwrap();
                }
            }
        }

        let path = format!("{repo_dir}/tests/images/opsz_overlay.png");
        overlay.save_png(&path).unwrap();
        eprintln!("Saved {path}");
        eprintln!();
        eprintln!("Red = opsz OFF (body default, wider glyphs)");
        eprintln!("Green = opsz ON (display, narrower glyphs)");
        eprintln!("Yellow = overlap");
    }
}

fn render_buffer(
    buffer: &mut Buffer,
    font_system: &mut FontSystem,
    canvas_w: u32,
    canvas_h: u32,
) -> Pixmap {
    let mut pixmap = Pixmap::new(canvas_w, canvas_h).unwrap();
    pixmap.fill(tiny_skia::Color::WHITE);
    let mut swash_cache = SwashCache::new();
    let text_color = Color::rgb(0, 0, 0);

    buffer.draw(
        font_system,
        &mut swash_cache,
        text_color,
        |x, y, w, h, color| {
            let mut paint = Paint {
                anti_alias: true,
                ..Paint::default()
            };
            paint.set_color_rgba8(color.r(), color.g(), color.b(), color.a());
            if let Some(rect) = Rect::from_xywh(x as f32, y as f32, w as f32, h as f32) {
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
        },
    );

    pixmap
}
