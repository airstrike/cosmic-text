// Bench: where should text-decoration geometry be resolved?
//
// Three states, two scenarios. The states:
//
//   current  — decoration baked into `LayoutLine.decorations` by shape();
//              a decoration change reshapes the line (text_decoration is in
//              the shape cache key / dirty path today).
//   A        — decoration removed from shaping; resolved LIVE from the line's
//              AttrsList + a per-font metric cache, every draw. A change is a
//              plain attrs mutation + redraw, never a reshape.
//   B        — decoration removed from shaping but still cached in a layout-
//              side overlay, rebuilt by a decoration-only recompute pass on
//              change (no reshape). Per-draw read is identical to `current`.
//
// Scenario 1 (perframe): steady-state draw of a static decorated doc — the
// scroll case. `current` and `B` both read the baked list; `A` resolves live.
//
// Scenario 2 (toggle): one decoration toggle on one line — the hovered-link
// case. `current` reshapes; `A` mutates attrs; `B` mutates + recomputes the
// one line's decoration overlay.

use cosmic_text as ct;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use ct::{
    Attrs, AttrsList, AttrsOverride, Buffer, Color, FontSystem, GlyphDecorationData, Metrics,
    Override, Shaping, TextDecoration, UnderlineStyle,
};
use std::collections::HashMap;
use std::hint::black_box;
use std::ops::Range;

struct Doc {
    /// Whole-document text; used by the real-pipeline bench below.
    #[allow(dead_code)]
    full_text: String,
    lines: Vec<String>,
    /// decorated byte ranges per line, relative to that line's start
    decorations: Vec<Vec<Range<usize>>>,
}

fn build_doc(n_lines: usize, spans_per_line: usize) -> Doc {
    let plain = "lorem ipsum ";
    let deco = "dolorsit";
    let mut lines = Vec::with_capacity(n_lines);
    let mut decorations = Vec::with_capacity(n_lines);
    for _ in 0..n_lines {
        let mut s = String::new();
        let mut ranges = Vec::with_capacity(spans_per_line);
        for _ in 0..spans_per_line {
            s.push_str(plain);
            let start = s.len();
            s.push_str(deco);
            ranges.push(start..s.len());
            s.push(' ');
        }
        s.push_str(plain);
        lines.push(s);
        decorations.push(ranges);
    }
    Doc {
        full_text: lines.join("\n"),
        lines,
        decorations,
    }
}

fn underline_attrs() -> Attrs<'static> {
    Attrs {
        text_decoration: TextDecoration {
            underline: UnderlineStyle::Single,
            ..TextDecoration::new()
        },
        ..Attrs::new()
    }
}

fn underline_override() -> AttrsOverride {
    AttrsOverride {
        text_decoration: Override::Set(TextDecoration {
            underline: UnderlineStyle::Single,
            ..TextDecoration::new()
        }),
        ..AttrsOverride::default()
    }
}

fn color_override() -> AttrsOverride {
    AttrsOverride {
        color: Override::Set(Some(Color::rgb(0x42, 0x85, 0xf4))),
        ..AttrsOverride::default()
    }
}

/// Build a shaped buffer (baked decorations available) + the parallel
/// per-line AttrsLists that states A and B query, + the per-font metric
/// cache seeded once from the baked decorations.
fn setup(
    fs: &mut FontSystem,
    doc: &Doc,
) -> (
    Buffer,
    Vec<AttrsList>,
    HashMap<ct::fontdb::ID, GlyphDecorationData>,
) {
    let base = Attrs::new();
    let under = underline_attrs();

    // rich-text spans, newline-separated lines
    let mut segs: Vec<(String, bool)> = Vec::new();
    for (li, line) in doc.lines.iter().enumerate() {
        let mut pos = 0usize;
        for r in &doc.decorations[li] {
            if r.start > pos {
                segs.push((line[pos..r.start].to_string(), false));
            }
            segs.push((line[r.clone()].to_string(), true));
            pos = r.end;
        }
        if pos < line.len() {
            segs.push((line[pos..].to_string(), false));
        }
        if li + 1 < doc.lines.len() {
            segs.push(("\n".to_string(), false));
        }
    }
    let spans: Vec<(&str, Attrs)> = segs
        .iter()
        .map(|(s, d)| (s.as_str(), if *d { under.clone() } else { base.clone() }))
        .collect();

    let mut buffer = Buffer::new(fs, Metrics::new(14.0, 20.0));
    buffer.set_size(Some(800.0), None);
    buffer.set_rich_text(spans, &base, Shaping::Advanced, None);
    buffer.shape_until_scroll(fs, false);

    // parallel per-line override lists (the source of truth for A and B)
    let over = underline_override();
    let line_lists: Vec<AttrsList> = doc
        .decorations
        .iter()
        .map(|ranges| {
            let mut al = AttrsList::new(&base);
            for r in ranges {
                al.add_span(r.clone(), &over);
            }
            al
        })
        .collect();

    // per-font metric cache: in A/B, decoration_metrics() is computed once
    // per font and cached. Seed it from the baked GlyphDecorationData so the
    // per-frame cost we measure is purely the lookup.
    let mut cache: HashMap<ct::fontdb::ID, GlyphDecorationData> = HashMap::new();
    for run in buffer.layout_runs() {
        for d in run.decorations {
            if let Some(g) = run.glyphs.get(d.glyph_range.start) {
                cache.entry(g.font_id).or_insert_with(|| d.data.clone());
            }
        }
    }

    (buffer, line_lists, cache)
}

/// current / B per-frame: read baked decorations, compute rect extents.
fn draw_baked(buffer: &Buffer) -> f32 {
    let mut sink = 0.0f32;
    for run in buffer.layout_runs() {
        for d in run.decorations {
            let glyphs = &run.glyphs[d.glyph_range.clone()];
            if glyphs.is_empty() {
                continue;
            }
            let mut x_min = f32::MAX;
            let mut x_max = f32::MIN;
            for g in glyphs {
                x_min = x_min.min(g.x);
                x_max = x_max.max(g.x + g.w);
            }
            let thickness = d.data.underline_metrics.thickness * d.font_size;
            let y = run.line_y + d.data.underline_metrics.offset * d.font_size;
            sink += (x_max - x_min) + thickness + y;
        }
    }
    sink
}

/// A per-frame: resolve decoration ranges live from the line's AttrsList,
/// map byte ranges to glyphs, pull metrics from the per-font cache.
fn draw_live(
    buffer: &Buffer,
    line_lists: &[AttrsList],
    cache: &HashMap<ct::fontdb::ID, GlyphDecorationData>,
) -> f32 {
    let mut sink = 0.0f32;
    for run in buffer.layout_runs() {
        let al = &line_lists[run.line_i];
        for (range, over) in al.spans_iter() {
            let td = match &over.text_decoration {
                Override::Set(td) if td.has_decoration() => td,
                _ => continue,
            };
            let _ = td;
            let mut x_min = f32::MAX;
            let mut x_max = f32::MIN;
            let mut font = None;
            let mut font_size = 0.0f32;
            for g in run.glyphs {
                if g.start >= range.start && g.end <= range.end {
                    x_min = x_min.min(g.x);
                    x_max = x_max.max(g.x + g.w);
                    if font.is_none() {
                        font = Some(g.font_id);
                        font_size = g.font_size;
                    }
                }
            }
            if let Some(fid) = font {
                if let Some(m) = cache.get(&fid) {
                    let thickness = m.underline_metrics.thickness * font_size;
                    let y = run.line_y + m.underline_metrics.offset * font_size;
                    sink += (x_max - x_min) + thickness + y;
                }
            }
        }
    }
    sink
}

/// A per-frame, optimized: single ordered merge-walk of glyphs against
/// the (sorted) override spans — O(glyphs + spans) per run, the way a real
/// draw-time resolver would do it.
fn draw_live_opt(
    buffer: &Buffer,
    line_lists: &[AttrsList],
    cache: &HashMap<ct::fontdb::ID, GlyphDecorationData>,
) -> f32 {
    let mut sink = 0.0f32;
    for run in buffer.layout_runs() {
        let al = &line_lists[run.line_i];
        let glyphs = run.glyphs;
        let mut gi = 0usize;
        for (range, over) in al.spans_iter() {
            match &over.text_decoration {
                Override::Set(td) if td.has_decoration() => {}
                _ => continue,
            }
            while gi < glyphs.len() && glyphs[gi].end <= range.start {
                gi += 1;
            }
            let mut j = gi;
            let mut x_min = f32::MAX;
            let mut x_max = f32::MIN;
            let mut font = None;
            let mut font_size = 0.0f32;
            while j < glyphs.len() && glyphs[j].start < range.end {
                let g = &glyphs[j];
                if g.start >= range.start && g.end <= range.end {
                    x_min = x_min.min(g.x);
                    x_max = x_max.max(g.x + g.w);
                    if font.is_none() {
                        font = Some(g.font_id);
                        font_size = g.font_size;
                    }
                }
                j += 1;
            }
            if let Some(fid) = font {
                if let Some(m) = cache.get(&fid) {
                    let thickness = m.underline_metrics.thickness * font_size;
                    let y = run.line_y + m.underline_metrics.offset * font_size;
                    sink += (x_max - x_min) + thickness + y;
                }
            }
        }
    }
    sink
}

fn bench_perframe(c: &mut Criterion) {
    let mut group = c.benchmark_group("decoration/perframe");
    for (n_lines, spans) in [(80, 1usize), (80, 6), (400, 6)] {
        let id = format!("{n_lines}x{spans}");
        let mut fs = FontSystem::new();
        let doc = build_doc(n_lines, spans);
        let (buffer, line_lists, cache) = setup(&mut fs, &doc);

        group.bench_function(BenchmarkId::new("current_baked", &id), |b| {
            b.iter(|| black_box(draw_baked(black_box(&buffer))))
        });
        // B's per-draw read is the same baked path as `current` by construction.
        group.bench_function(BenchmarkId::new("B_baked", &id), |b| {
            b.iter(|| black_box(draw_baked(black_box(&buffer))))
        });
        group.bench_function(BenchmarkId::new("A_live_naive", &id), |b| {
            b.iter(|| black_box(draw_live(black_box(&buffer), &line_lists, &cache)))
        });
        group.bench_function(BenchmarkId::new("A_live_opt", &id), |b| {
            b.iter(|| black_box(draw_live_opt(black_box(&buffer), &line_lists, &cache)))
        });
    }
    group.finish();
}

fn bench_toggle(c: &mut Criterion) {
    let n_lines = 80usize;
    let spans = 6usize;
    let target = n_lines / 2;

    let mut group = c.benchmark_group("decoration/toggle");
    group.sample_size(50);

    // ---- current: toggle reshapes the line ----
    {
        let mut fs = FontSystem::new();
        let doc = build_doc(n_lines, spans);
        let (mut buffer, _line_lists, _cache) = setup(&mut fs, &doc);
        let base = Attrs::new();

        let list_off = AttrsList::new(&base);
        let over = underline_override();
        let mut list_on = AttrsList::new(&base);
        for r in &doc.decorations[target] {
            list_on.add_span(r.clone(), &over);
        }

        let mut toggle = false;
        group.bench_function("current_reshape", |b| {
            b.iter(|| {
                toggle = !toggle;
                let list = if toggle {
                    list_on.clone()
                } else {
                    list_off.clone()
                };
                buffer.lines[target].set_attrs_list(list);
                buffer.shape_until_scroll(&mut fs, false);
                black_box(buffer.layout_runs().count());
            })
        });
    }

    // ---- A: toggle is a plain attrs mutation, no reshape ----
    {
        let mut fs = FontSystem::new();
        let doc = build_doc(n_lines, spans);
        let (_buffer, mut line_lists, _cache) = setup(&mut fs, &doc);
        let over = underline_override();
        let ranges = doc.decorations[target].clone();

        let mut toggle = false;
        group.bench_function("A_mutate", |b| {
            b.iter(|| {
                toggle = !toggle;
                let al = &mut line_lists[target];
                al.clear_spans();
                if toggle {
                    for r in &ranges {
                        al.add_span(r.clone(), &over);
                    }
                }
                black_box(&*al);
            })
        });
    }

    // ---- B: mutation + one-line decoration recompute, no reshape ----
    {
        let mut fs = FontSystem::new();
        let doc = build_doc(n_lines, spans);
        let (buffer, mut line_lists, cache) = setup(&mut fs, &doc);
        let over = underline_override();
        let ranges = doc.decorations[target].clone();

        let mut toggle = false;
        group.bench_function("B_recompute", |b| {
            b.iter(|| {
                toggle = !toggle;
                let al = &mut line_lists[target];
                al.clear_spans();
                if toggle {
                    for r in &ranges {
                        al.add_span(r.clone(), &over);
                    }
                }
                // rebuild the cached overlay for just this line from existing glyphs
                let mut overlay: Vec<(Range<usize>, f32, f32)> = Vec::new();
                for run in buffer.layout_runs() {
                    if run.line_i != target {
                        continue;
                    }
                    for (range, ov) in al.spans_iter() {
                        match &ov.text_decoration {
                            Override::Set(td) if td.has_decoration() => {}
                            _ => continue,
                        }
                        let mut x_min = f32::MAX;
                        let mut x_max = f32::MIN;
                        let mut font = None;
                        let mut font_size = 0.0f32;
                        for g in run.glyphs {
                            if g.start >= range.start && g.end <= range.end {
                                x_min = x_min.min(g.x);
                                x_max = x_max.max(g.x + g.w);
                                if font.is_none() {
                                    font = Some(g.font_id);
                                    font_size = g.font_size;
                                }
                            }
                        }
                        if let Some(fid) = font {
                            if let Some(m) = cache.get(&fid) {
                                overlay.push((
                                    range.clone(),
                                    x_max - x_min,
                                    m.underline_metrics.thickness * font_size,
                                ));
                            }
                        }
                    }
                }
                black_box(overlay);
            })
        });
    }

    group.finish();
}

/// Status-quo + Approach-A cost of a COLOR-only change on one span.
///
/// `current_reshape` mutates the line's `AttrsList` with a color-only override
/// and reshapes (the status quo: `color` is in the shape cache key and in the
/// `set_attrs_list` equality check). `A_mutate` is the plain attrs mutation A
/// reduces it to.
fn bench_color_toggle(c: &mut Criterion) {
    let n_lines = 80usize;
    let spans = 6usize;
    let target = n_lines / 2;

    let mut group = c.benchmark_group("color/toggle");
    group.sample_size(50);

    {
        let mut fs = FontSystem::new();
        let doc = build_doc(n_lines, spans);
        let (mut buffer, _line_lists, _cache) = setup(&mut fs, &doc);
        let base = Attrs::new();

        let list_off = AttrsList::new(&base);
        let over = color_override();
        let mut list_on = AttrsList::new(&base);
        for r in &doc.decorations[target] {
            list_on.add_span(r.clone(), &over);
        }

        let mut toggle = false;
        group.bench_function("current_reshape", |b| {
            b.iter(|| {
                toggle = !toggle;
                let list = if toggle {
                    list_on.clone()
                } else {
                    list_off.clone()
                };
                buffer.lines[target].set_attrs_list(list);
                buffer.shape_until_scroll(&mut fs, false);
                black_box(buffer.layout_runs().count());
            })
        });
    }

    {
        let mut fs = FontSystem::new();
        let doc = build_doc(n_lines, spans);
        let (_buffer, mut line_lists, _cache) = setup(&mut fs, &doc);
        let over = color_override();
        let ranges = doc.decorations[target].clone();

        let mut toggle = false;
        group.bench_function("A_mutate", |b| {
            b.iter(|| {
                toggle = !toggle;
                let al = &mut line_lists[target];
                al.clear_spans();
                if toggle {
                    for r in &ranges {
                        al.add_span(r.clone(), &over);
                    }
                }
                black_box(&*al);
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_perframe, bench_toggle, bench_color_toggle);
criterion_main!(benches);
