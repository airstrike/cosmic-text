// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::hash::{Hash, Hasher};
use core::ops::Range;
use rangemap::RangeMap;
use smol_str::SmolStr;

use crate::{CacheKeyFlags, Metrics};

pub use fontdb::{Family, Stretch, Style, Weight};

/// Optical size setting for variable fonts with an `opsz` axis.
#[derive(Clone, Copy, Debug, Default)]
pub enum OpticalSize {
    /// Automatically set `opsz` to match the font size.
    Auto,
    /// Set `opsz` to a specific value, independent of font size.
    Fixed(f32),
    /// Disable optical sizing entirely (default).
    #[default]
    None,
}

impl OpticalSize {
    pub fn resolve(self, font_size: f32) -> Option<f32> {
        match self {
            Self::Auto => Some(font_size),
            Self::Fixed(v) => Some(v),
            Self::None => None,
        }
    }

    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
    }

    /// Resolve to `Option<f32>` without knowing the rendered font size.
    /// `Fixed(v)` resolves to `Some(v)`; `Auto` and `None` both resolve to
    /// `None`. Use this during font fallback selection where the final
    /// rendered size is not yet known.
    pub fn resolve_fixed(self) -> Option<f32> {
        match self {
            Self::Fixed(v) => Some(v),
            _ => None,
        }
    }
}

impl PartialEq for OpticalSize {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Auto, Self::Auto) | (Self::None, Self::None) => true,
            (Self::Fixed(a), Self::Fixed(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

impl Eq for OpticalSize {}

impl core::hash::Hash for OpticalSize {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        if let Self::Fixed(v) = self {
            v.to_bits().hash(state);
        }
    }
}

impl PartialOrd for OpticalSize {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OpticalSize {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match (self, other) {
            (Self::Auto, Self::Auto) | (Self::None, Self::None) => core::cmp::Ordering::Equal,
            (Self::Auto, _) => core::cmp::Ordering::Less,
            (_, Self::Auto) => core::cmp::Ordering::Greater,
            (Self::None, _) => core::cmp::Ordering::Greater,
            (_, Self::None) => core::cmp::Ordering::Less,
            (Self::Fixed(a), Self::Fixed(b)) => a.to_bits().cmp(&b.to_bits()),
        }
    }
}

/// Text color
#[derive(Clone, Copy, Debug, PartialOrd, Ord, Eq, Hash, PartialEq)]
pub struct Color(pub u32);

impl Color {
    /// Create new color with red, green, and blue components
    #[inline]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 0xFF)
    }

    /// Create new color with red, green, blue, and alpha components
    #[inline]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self(((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    /// Get a tuple over all of the attributes, in `(r, g, b, a)` order.
    #[inline]
    pub const fn as_rgba_tuple(self) -> (u8, u8, u8, u8) {
        (self.r(), self.g(), self.b(), self.a())
    }

    /// Get an array over all of the components, in `[r, g, b, a]` order.
    #[inline]
    pub const fn as_rgba(self) -> [u8; 4] {
        [self.r(), self.g(), self.b(), self.a()]
    }

    /// Get the red component
    #[inline]
    pub const fn r(&self) -> u8 {
        ((self.0 & 0x00_FF_00_00) >> 16) as u8
    }

    /// Get the green component
    #[inline]
    pub const fn g(&self) -> u8 {
        ((self.0 & 0x00_00_FF_00) >> 8) as u8
    }

    /// Get the blue component
    #[inline]
    pub const fn b(&self) -> u8 {
        (self.0 & 0x00_00_00_FF) as u8
    }

    /// Get the alpha component
    #[inline]
    pub const fn a(&self) -> u8 {
        ((self.0 & 0xFF_00_00_00) >> 24) as u8
    }
}

/// An owned version of [`Family`]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FamilyOwned {
    Name(SmolStr),
    Serif,
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

impl FamilyOwned {
    pub fn new(family: Family) -> Self {
        match family {
            Family::Name(name) => Self::Name(SmolStr::from(name)),
            Family::Serif => Self::Serif,
            Family::SansSerif => Self::SansSerif,
            Family::Cursive => Self::Cursive,
            Family::Fantasy => Self::Fantasy,
            Family::Monospace => Self::Monospace,
        }
    }

    pub fn as_family(&self) -> Family<'_> {
        match self {
            Self::Name(name) => Family::Name(name),
            Self::Serif => Family::Serif,
            Self::SansSerif => Family::SansSerif,
            Self::Cursive => Family::Cursive,
            Self::Fantasy => Family::Fantasy,
            Self::Monospace => Family::Monospace,
        }
    }
}

/// Metrics, but implementing Eq and Hash using u32 representation of f32
//TODO: what are the edge cases of this?
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CacheMetrics {
    font_size_bits: u32,
    line_height_bits: u32,
}

impl From<Metrics> for CacheMetrics {
    fn from(metrics: Metrics) -> Self {
        Self {
            font_size_bits: metrics.font_size.to_bits(),
            line_height_bits: metrics.line_height.to_bits(),
        }
    }
}

impl From<CacheMetrics> for Metrics {
    fn from(metrics: CacheMetrics) -> Self {
        Self {
            font_size: f32::from_bits(metrics.font_size_bits),
            line_height: f32::from_bits(metrics.line_height_bits),
        }
    }
}
/// A 4-byte `OpenType` feature tag identifier
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FeatureTag([u8; 4]);

impl FeatureTag {
    pub const fn new(tag: &[u8; 4]) -> Self {
        Self(*tag)
    }

    /// Kerning adjusts spacing between specific character pairs
    pub const KERNING: Self = Self::new(b"kern");
    /// Standard ligatures (fi, fl, etc.)
    pub const STANDARD_LIGATURES: Self = Self::new(b"liga");
    /// Contextual ligatures (context-dependent ligatures)
    pub const CONTEXTUAL_LIGATURES: Self = Self::new(b"clig");
    /// Contextual alternates (glyph substitutions based on context)
    pub const CONTEXTUAL_ALTERNATES: Self = Self::new(b"calt");
    /// Discretionary ligatures (optional stylistic ligatures)
    pub const DISCRETIONARY_LIGATURES: Self = Self::new(b"dlig");
    /// Small caps (lowercase to small capitals)
    pub const SMALL_CAPS: Self = Self::new(b"smcp");
    /// All small caps (uppercase and lowercase to small capitals)
    pub const ALL_SMALL_CAPS: Self = Self::new(b"c2sc");
    /// Stylistic Set 1 (font-specific alternate glyphs)
    pub const STYLISTIC_SET_1: Self = Self::new(b"ss01");
    /// Stylistic Set 2 (font-specific alternate glyphs)
    pub const STYLISTIC_SET_2: Self = Self::new(b"ss02");

    pub const fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Feature {
    pub tag: FeatureTag,
    pub value: u32,
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct FontFeatures {
    pub features: Vec<Feature>,
}

impl FontFeatures {
    pub const fn new() -> Self {
        Self {
            features: Vec::new(),
        }
    }

    pub fn set(&mut self, tag: FeatureTag, value: u32) -> &mut Self {
        self.features.push(Feature { tag, value });
        self
    }

    /// Enable a feature (set to 1)
    pub fn enable(&mut self, tag: FeatureTag) -> &mut Self {
        self.set(tag, 1)
    }

    /// Disable a feature (set to 0)
    pub fn disable(&mut self, tag: FeatureTag) -> &mut Self {
        self.set(tag, 0)
    }
}

/// A wrapper for letter spacing to get around that f32 doesn't implement Eq and Hash
#[derive(Clone, Copy, Debug)]
pub struct LetterSpacing(pub f32);

impl PartialEq for LetterSpacing {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() {
            other.0.is_nan()
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for LetterSpacing {}

impl Hash for LetterSpacing {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        const CANONICAL_NAN_BITS: u32 = 0x7fc0_0000;

        let bits = if self.0.is_nan() {
            CANONICAL_NAN_BITS
        } else {
            // Add +0.0 to canonicalize -0.0 to +0.0
            (self.0 + 0.0).to_bits()
        };

        bits.hash(hasher);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    // TODO: Wavy
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct TextDecoration {
    pub underline: UnderlineStyle,
    pub underline_color_opt: Option<Color>,
    pub strikethrough: bool,
    pub strikethrough_color_opt: Option<Color>,
    pub overline: bool,
    pub overline_color_opt: Option<Color>,
}

impl TextDecoration {
    pub const fn new() -> Self {
        Self {
            underline: UnderlineStyle::None,
            underline_color_opt: None,
            strikethrough: false,
            strikethrough_color_opt: None,
            overline: false,
            overline_color_opt: None,
        }
    }

    pub const fn has_decoration(&self) -> bool {
        !matches!(self.underline, UnderlineStyle::None) || self.strikethrough || self.overline
    }
}

/// Offset and thickness for a text decoration line, in EM units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DecorationMetrics {
    /// Offset from baseline in EM units
    pub offset: f32,
    /// Thickness in EM units
    pub thickness: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GlyphDecorationData {
    /// The text decoration configuration from the user
    pub text_decoration: TextDecoration,
    /// Underline offset and thickness from the font
    pub underline_metrics: DecorationMetrics,
    /// Strikethrough offset and thickness from the font
    pub strikethrough_metrics: DecorationMetrics,
    /// Font ascent in EM units (ascent / upem).
    /// Used for overline positioning
    pub ascent: f32,
}

/// Text attributes
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Attrs<'a> {
    //TODO: should this be an option?
    pub color_opt: Option<Color>,
    pub family: Family<'a>,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight,
    pub metadata: usize,
    pub cache_key_flags: CacheKeyFlags,
    pub metrics_opt: Option<CacheMetrics>,
    /// Letter spacing (tracking) in EM
    pub letter_spacing_opt: Option<LetterSpacing>,
    pub font_features: FontFeatures,
    pub text_decoration: TextDecoration,
    pub optical_size: OpticalSize,
}

impl<'a> Attrs<'a> {
    /// Create a new set of attributes with sane defaults
    ///
    /// This defaults to a regular Sans-Serif font.
    pub const fn new() -> Self {
        Self {
            color_opt: None,
            family: Family::SansSerif,
            stretch: Stretch::Normal,
            style: Style::Normal,
            weight: Weight::NORMAL,
            metadata: 0,
            cache_key_flags: CacheKeyFlags::empty(),
            metrics_opt: None,
            letter_spacing_opt: None,
            font_features: FontFeatures::new(),
            text_decoration: TextDecoration::new(),
            optical_size: OpticalSize::None,
        }
    }

    /// Set [Color]
    pub const fn color(mut self, color: Color) -> Self {
        self.color_opt = Some(color);
        self
    }

    /// Set [Family]
    pub const fn family(mut self, family: Family<'a>) -> Self {
        self.family = family;
        self
    }

    /// Set [Stretch]
    pub const fn stretch(mut self, stretch: Stretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Set [Style]
    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set [Weight]
    pub const fn weight(mut self, weight: Weight) -> Self {
        self.weight = weight;
        self
    }

    /// Set metadata
    pub const fn metadata(mut self, metadata: usize) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set [`CacheKeyFlags`]
    pub const fn cache_key_flags(mut self, cache_key_flags: CacheKeyFlags) -> Self {
        self.cache_key_flags = cache_key_flags;
        self
    }

    /// Set [`Metrics`], overriding values in buffer
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        self.metrics_opt = Some(metrics.into());
        self
    }

    /// Set letter spacing (tracking) in EM
    pub const fn letter_spacing(mut self, letter_spacing: f32) -> Self {
        self.letter_spacing_opt = Some(LetterSpacing(letter_spacing));
        self
    }

    /// Set [`FontFeatures`]
    pub fn font_features(mut self, font_features: FontFeatures) -> Self {
        self.font_features = font_features;
        self
    }

    /// Enable or disable automatic optical sizing.
    /// When true, the font's `opsz` axis is set to match the rendered font size.
    pub fn optical_sizing(mut self, enabled: bool) -> Self {
        self.optical_size = if enabled {
            OpticalSize::Auto
        } else {
            OpticalSize::None
        };
        self
    }

    /// Set a specific [`OpticalSize`] value.
    pub fn optical_size(mut self, optical_size: OpticalSize) -> Self {
        self.optical_size = optical_size;
        self
    }

    pub const fn underline(mut self, style: UnderlineStyle) -> Self {
        self.text_decoration.underline = style;
        self
    }

    pub const fn underline_color(mut self, color: Color) -> Self {
        self.text_decoration.underline_color_opt = Some(color);
        self
    }

    pub const fn strikethrough(mut self) -> Self {
        self.text_decoration.strikethrough = true;
        self
    }

    pub const fn strikethrough_color(mut self, color: Color) -> Self {
        self.text_decoration.strikethrough_color_opt = Some(color);
        self
    }

    pub const fn overline(mut self) -> Self {
        self.text_decoration.overline = true;
        self
    }

    pub const fn overline_color(mut self, color: Color) -> Self {
        self.text_decoration.overline_color_opt = Some(color);
        self
    }

    /// Check if this set of attributes can be shaped with another
    pub fn compatible(&self, other: &Self) -> bool {
        self.family == other.family
            && self.stretch == other.stretch
            && self.style == other.style
            && self.weight == other.weight
    }
}

/// Font-specific part of [`Attrs`] to be used for matching
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FontMatchAttrs {
    family: FamilyOwned,
    stretch: Stretch,
    style: Style,
    weight: Weight,
}

impl<'a> From<&Attrs<'a>> for FontMatchAttrs {
    fn from(attrs: &Attrs<'a>) -> Self {
        Self {
            family: FamilyOwned::new(attrs.family),
            stretch: attrs.stretch,
            style: attrs.style,
            weight: attrs.weight,
        }
    }
}

/// Attributes that determine shaped output.
///
/// Shaped runs are reusable wherever these are equal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ShapeAttrs {
    pub family_owned: FamilyOwned,
    pub stretch: Stretch,
    pub style: Style,
    pub weight: Weight,
    pub metadata: usize,
    pub cache_key_flags: CacheKeyFlags,
    pub metrics_opt: Option<CacheMetrics>,
    /// Letter spacing (tracking) in EM
    pub letter_spacing_opt: Option<LetterSpacing>,
    pub font_features: FontFeatures,
    pub optical_size: OpticalSize,
}

/// Render attributes, which are resolved at layout and do
/// not involve reshaping.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct RenderAttrs {
    //TODO: should this be an option?
    pub color_opt: Option<Color>,
    pub text_decoration: TextDecoration,
}

/// An owned version of [`Attrs`]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AttrsOwned {
    pub shape: ShapeAttrs,
    pub render: RenderAttrs,
}

impl ShapeAttrs {
    pub fn new(attrs: &Attrs) -> Self {
        Self {
            family_owned: FamilyOwned::new(attrs.family),
            stretch: attrs.stretch,
            style: attrs.style,
            weight: attrs.weight,
            metadata: attrs.metadata,
            cache_key_flags: attrs.cache_key_flags,
            metrics_opt: attrs.metrics_opt,
            letter_spacing_opt: attrs.letter_spacing_opt,
            font_features: attrs.font_features.clone(),
            optical_size: attrs.optical_size,
        }
    }
}

impl RenderAttrs {
    pub const fn new(attrs: &Attrs) -> Self {
        Self {
            color_opt: attrs.color_opt,
            text_decoration: attrs.text_decoration,
        }
    }
}

impl AttrsOwned {
    pub fn new(attrs: &Attrs) -> Self {
        Self {
            shape: ShapeAttrs::new(attrs),
            render: RenderAttrs::new(attrs),
        }
    }

    pub fn as_attrs(&self) -> Attrs<'_> {
        Attrs {
            color_opt: self.render.color_opt,
            family: self.shape.family_owned.as_family(),
            stretch: self.shape.stretch,
            style: self.shape.style,
            weight: self.shape.weight,
            metadata: self.shape.metadata,
            cache_key_flags: self.shape.cache_key_flags,
            metrics_opt: self.shape.metrics_opt,
            letter_spacing_opt: self.shape.letter_spacing_opt,
            font_features: self.shape.font_features.clone(),
            text_decoration: self.render.text_decoration,
            optical_size: self.shape.optical_size,
        }
    }
}

/// List of text attributes to apply to a line
//TODO: have this clean up the spans when changes are made
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AttrsList {
    defaults: AttrsOwned,
    pub(crate) spans: RangeMap<usize, AttrsOwned>,
}

impl AttrsList {
    /// Create a new attributes list with a set of default [Attrs]
    pub fn new(defaults: &Attrs) -> Self {
        Self {
            defaults: AttrsOwned::new(defaults),
            spans: RangeMap::new(),
        }
    }

    /// Get the default [Attrs]
    pub fn defaults(&self) -> Attrs<'_> {
        self.defaults.as_attrs()
    }

    /// True if the two lists differ only in [`RenderAttrs`]. Spans that carry
    /// only render changes (otherwise equal to the defaults) are ignored.
    pub fn eq_shape_attrs(&self, other: &Self) -> bool {
        self.defaults.shape == other.defaults.shape && self.shape_spans().eq(other.shape_spans())
    }

    /// The spans that differ from the defaults in shape, projected onto
    /// [`ShapeAttrs`]
    fn shape_spans(&self) -> impl Iterator<Item = (&Range<usize>, &ShapeAttrs)> + '_ {
        self.spans_iter()
            .filter(|(_, attrs)| attrs.shape != self.defaults.shape)
            .map(|(range, attrs)| (range, &attrs.shape))
    }

    /// Get the current attribute spans
    pub fn spans(&self) -> Vec<(&Range<usize>, &AttrsOwned)> {
        self.spans_iter().collect()
    }

    /// Get an iterator over the current attribute spans
    pub fn spans_iter(&self) -> impl Iterator<Item = (&Range<usize>, &AttrsOwned)> + '_ {
        self.spans.iter()
    }

    /// Clear the current attribute spans
    pub fn clear_spans(&mut self) {
        self.spans.clear();
    }

    /// Add an attribute span, removes any previous matching parts of spans
    pub fn add_span(&mut self, range: Range<usize>, attrs: &Attrs) {
        //do not support 1..1 or 2..1 even if by accident.
        if range.is_empty() {
            return;
        }

        self.spans.insert(range, AttrsOwned::new(attrs));
    }

    /// Get the attribute span for an index
    ///
    /// This returns a span that contains the index
    pub fn get_span(&self, index: usize) -> Attrs<'_> {
        self.spans
            .get(&index)
            .map(|v| v.as_attrs())
            .unwrap_or(self.defaults.as_attrs())
    }

    /// The color at `index` and a contiguous byte range it holds over, so a
    /// caller walking mostly forward can resolve a run of glyphs with one
    /// lookup.
    ///
    /// For an `index` in the gap between spans, the range starts at `index`
    /// rather than at the start of the gap, since finding the true start
    /// would cost a second lookup and a forward walk never revisits it.
    pub(crate) fn color_run(&self, index: usize) -> (Range<usize>, Option<Color>) {
        match self.spans.overlapping(index..usize::MAX).next() {
            Some((range, attrs)) if range.contains(&index) => {
                (range.clone(), attrs.render.color_opt)
            }
            Some((range, _)) => (index..range.start, self.defaults.render.color_opt),
            None => (index..usize::MAX, self.defaults.render.color_opt),
        }
    }

    /// Split attributes list at an offset
    #[allow(clippy::missing_panics_doc)]
    pub fn split_off(&mut self, index: usize) -> Self {
        let mut new = Self::new(&self.defaults.as_attrs());
        let mut removes = Vec::new();

        //get the keys we need to remove or fix.
        for span in self.spans.iter() {
            if span.0.end <= index {
                continue;
            }

            if span.0.start >= index {
                removes.push((span.0.clone(), false));
            } else {
                removes.push((span.0.clone(), true));
            }
        }

        for (key, resize) in removes {
            let (range, attrs) = self
                .spans
                .get_key_value(&key.start)
                .map(|v| (v.0.clone(), v.1.clone()))
                .expect("attrs span not found");
            self.spans.remove(key);

            if resize {
                new.spans.insert(0..range.end - index, attrs.clone());
                self.spans.insert(range.start..index, attrs);
            } else {
                new.spans
                    .insert(range.start - index..range.end - index, attrs);
            }
        }
        new
    }

    /// Resets the attributes with new defaults.
    pub(crate) fn reset(mut self, default: &Attrs) -> Self {
        self.defaults = AttrsOwned::new(default);
        self.spans.clear();
        self
    }
}
