use cosmic_text as ct;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_color_toggle(c: &mut Criterion) {
    let mut fs = ct::FontSystem::new();
    let mut buffer = ct::Buffer::new(&mut fs, ct::Metrics::new(14.0, 20.0));
    buffer.set_size(Some(600.0), None);

    let text = "The quick brown fox jumps over the lazy dog. ".repeat(8);
    buffer.set_text(&text, &ct::Attrs::new(), ct::Shaping::Advanced, None);
    buffer.shape_until_scroll(&mut fs, false);

    let plain = ct::AttrsList::new(&ct::Attrs::new());
    let mut red = ct::Attrs::new();
    red.color_opt = Some(ct::Color::rgb(0xFF, 0x00, 0x00));
    let mut colored = ct::AttrsList::new(&ct::Attrs::new());
    colored.add_span(0..20, &red);

    let mut on = false;
    c.bench_function("ShapeLine/Color Toggle", |b| {
        b.iter(|| {
            on = !on;
            buffer.lines[0].set_attrs_list(if on { colored.clone() } else { plain.clone() });
            buffer.shape_until_scroll(&mut fs, false);
            black_box(buffer.layout_runs().count());
        });
    });
}

criterion_group!(benches, bench_color_toggle);
criterion_main!(benches);
