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
    /// Resolve the optical size value given a font size.
    ///
    /// Returns `Some(value)` for the opsz axis, or `None` if disabled.
    pub fn resolve(self, font_size: f32) -> Option<f32> {
        match self {
            Self::Auto => Some(font_size),
            Self::Fixed(v) => Some(v),
            Self::None => None,
        }
    }

    /// Returns `true` if optical sizing is disabled.
    pub fn is_none(self) -> bool {
        matches!(self, Self::None)
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

/// A font variation axis setting (e.g. `wdth`, `slnt`, `GRAD`).
///
/// Each variation pairs a 4-byte OpenType axis tag with a continuous `f32` value.
/// Uses bit-level comparison for `Eq`/`Hash` since `f32` doesn't implement them.
#[derive(Clone, Copy, Debug)]
pub struct Variation {
    pub tag: FeatureTag,
    pub value: f32,
}

impl PartialEq for Variation {
    fn eq(&self, other: &Self) -> bool {
        self.tag == other.tag && self.value.to_bits() == other.value.to_bits()
    }
}

impl Eq for Variation {}

impl Hash for Variation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.tag.hash(state);
        self.value.to_bits().hash(state);
    }
}

/// A collection of font variation axis settings.
///
/// These control continuous axes on variable fonts (e.g. width, slant, grade)
/// and are passed through to the font instantiation layer (skrifa).
///
/// Note: `wght` and `opsz` are handled via dedicated [`Weight`] and [`OpticalSize`]
/// fields on [`Attrs`]. Use this for all other axes.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct FontVariations {
    pub variations: Vec<Variation>,
}

impl FontVariations {
    pub const fn new() -> Self {
        Self {
            variations: Vec::new(),
        }
    }

    /// Set a variation axis to a value.
    pub fn set(&mut self, tag: FeatureTag, value: f32) -> &mut Self {
        self.variations.push(Variation { tag, value });
        self
    }

    /// Compute a hash of all variations for use in fixed-size cache keys.
    ///
    /// Must be deterministic: the same variations must always produce the same
    /// hash within a process lifetime, since the hash is compared across
    /// shaping (font cache) and rendering (glyph cache) stages.
    pub fn cache_hash(&self) -> u64 {
        // Use FNV-1a for a fast, deterministic hash on all targets.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
        for v in &self.variations {
            h ^= v.tag.as_bytes()[0] as u64
                | (v.tag.as_bytes()[1] as u64) << 8
                | (v.tag.as_bytes()[2] as u64) << 16
                | (v.tag.as_bytes()[3] as u64) << 24;
            h = h.wrapping_mul(0x0100_0000_01b3);
            h ^= v.value.to_bits() as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h
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
    pub font_variations: FontVariations,
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
            font_variations: FontVariations::new(),
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

    /// Set [`FontVariations`]
    pub fn font_variations(mut self, font_variations: FontVariations) -> Self {
        self.font_variations = font_variations;
        self
    }

    /// Enable or disable optical sizing (CSS `font-optical-sizing`).
    /// When enabled, the `opsz` axis is set to match the font size.
    /// When disabled (default), the `opsz` axis is left at the font's default value.
    pub const fn optical_sizing(mut self, enabled: bool) -> Self {
        self.optical_size = if enabled {
            OpticalSize::Auto
        } else {
            OpticalSize::None
        };
        self
    }

    /// Set optical size to a specific value, `Auto`, or `None`.
    pub const fn optical_size(mut self, optical_size: OpticalSize) -> Self {
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

/// An owned version of [`Attrs`]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AttrsOwned {
    //TODO: should this be an option?
    pub color_opt: Option<Color>,
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
    pub font_variations: FontVariations,
    pub text_decoration: TextDecoration,
    pub optical_size: OpticalSize,
}

impl AttrsOwned {
    pub fn new(attrs: &Attrs) -> Self {
        Self {
            color_opt: attrs.color_opt,
            family_owned: FamilyOwned::new(attrs.family),
            stretch: attrs.stretch,
            style: attrs.style,
            weight: attrs.weight,
            metadata: attrs.metadata,
            cache_key_flags: attrs.cache_key_flags,
            metrics_opt: attrs.metrics_opt,
            letter_spacing_opt: attrs.letter_spacing_opt,
            font_features: attrs.font_features.clone(),
            font_variations: attrs.font_variations.clone(),
            text_decoration: attrs.text_decoration,
            optical_size: attrs.optical_size,
        }
    }

    pub fn as_attrs(&self) -> Attrs<'_> {
        Attrs {
            color_opt: self.color_opt,
            family: self.family_owned.as_family(),
            stretch: self.stretch,
            style: self.style,
            weight: self.weight,
            metadata: self.metadata,
            cache_key_flags: self.cache_key_flags,
            metrics_opt: self.metrics_opt,
            letter_spacing_opt: self.letter_spacing_opt,
            font_features: self.font_features.clone(),
            font_variations: self.font_variations.clone(),
            text_decoration: self.text_decoration,
            optical_size: self.optical_size,
        }
    }
}

/// One field of an [`AttrsOverride`]: either inherit the value from
/// the line's defaults, or set it explicitly.
///
/// Used as the building block for sparse span overrides. For fields
/// whose underlying [`Attrs`] type is already `Option<T>` (e.g.
/// `color_opt: Option<Color>`), wrapping that `Option` inside
/// `Override` yields a three-state semantic:
/// - [`Inherit`](Self::Inherit) — use the line defaults.
/// - `Set(None)` — force the field to "none" (e.g. clear color).
/// - `Set(Some(v))` — set the field to `v`.
///
/// For non-`Option` fields like `Weight`, only the two-state behavior
/// applies: `Inherit` or `Set(w)`.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub enum Override<T> {
    /// Inherit this field from the line's default attributes.
    #[default]
    Inherit,
    /// Override this field with the contained value.
    Set(T),
}

impl<T: Copy> Copy for Override<T> {}

impl<T> Override<T> {
    /// Returns `true` if this is a [`Set`](Self::Set) value.
    pub fn is_set(&self) -> bool {
        matches!(self, Self::Set(_))
    }

    /// Returns `true` if this is [`Inherit`](Self::Inherit).
    pub fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit)
    }

    /// Borrow the contained value if [`Set`](Self::Set).
    pub fn as_ref(&self) -> Override<&T> {
        match self {
            Self::Inherit => Override::Inherit,
            Self::Set(v) => Override::Set(v),
        }
    }

    /// Return the contained value if [`Set`](Self::Set), otherwise
    /// return the provided default.
    pub fn resolve_or(self, default: T) -> T {
        match self {
            Self::Inherit => default,
            Self::Set(v) => v,
        }
    }
}

/// A sparse override on top of an [`AttrsList`]'s default attributes.
///
/// Each field carries an [`Override<T>`] — either inheriting from the
/// line defaults or setting an explicit value. This lets per-span
/// overrides express *only* what differs from defaults, so changing
/// the line defaults automatically re-inherits for all unset fields.
///
/// See the type-level documentation on [`Override`] for the three-state
/// semantic that applies to fields whose underlying type is already
/// `Option<T>` (color, metrics, letter-spacing).
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct AttrsOverride {
    pub color: Override<Option<Color>>,
    pub metrics: Override<Option<CacheMetrics>>,
    pub letter_spacing: Override<Option<LetterSpacing>>,
    pub family: Override<FamilyOwned>,
    pub stretch: Override<Stretch>,
    pub style: Override<Style>,
    pub weight: Override<Weight>,
    pub metadata: Override<usize>,
    pub cache_key_flags: Override<CacheKeyFlags>,
    pub font_features: Override<FontFeatures>,
    pub font_variations: Override<FontVariations>,
    pub text_decoration: Override<TextDecoration>,
    pub optical_size: Override<OpticalSize>,
}

impl AttrsOverride {
    /// Compute the sparse override that maps `defaults` to `attrs`.
    ///
    /// For each field, returns [`Override::Inherit`] if the field is
    /// equal in both sides, [`Override::Set`] with the value from
    /// `attrs` otherwise.
    pub fn diff(defaults: &Attrs, attrs: &Attrs) -> Self {
        Self {
            color: if attrs.color_opt == defaults.color_opt {
                Override::Inherit
            } else {
                Override::Set(attrs.color_opt)
            },
            metrics: if attrs.metrics_opt == defaults.metrics_opt {
                Override::Inherit
            } else {
                Override::Set(attrs.metrics_opt)
            },
            letter_spacing: if attrs.letter_spacing_opt == defaults.letter_spacing_opt {
                Override::Inherit
            } else {
                Override::Set(attrs.letter_spacing_opt)
            },
            family: if attrs.family == defaults.family {
                Override::Inherit
            } else {
                Override::Set(FamilyOwned::new(attrs.family))
            },
            stretch: if attrs.stretch == defaults.stretch {
                Override::Inherit
            } else {
                Override::Set(attrs.stretch)
            },
            style: if attrs.style == defaults.style {
                Override::Inherit
            } else {
                Override::Set(attrs.style)
            },
            weight: if attrs.weight == defaults.weight {
                Override::Inherit
            } else {
                Override::Set(attrs.weight)
            },
            metadata: if attrs.metadata == defaults.metadata {
                Override::Inherit
            } else {
                Override::Set(attrs.metadata)
            },
            cache_key_flags: if attrs.cache_key_flags == defaults.cache_key_flags {
                Override::Inherit
            } else {
                Override::Set(attrs.cache_key_flags)
            },
            font_features: if attrs.font_features == defaults.font_features {
                Override::Inherit
            } else {
                Override::Set(attrs.font_features.clone())
            },
            font_variations: if attrs.font_variations == defaults.font_variations {
                Override::Inherit
            } else {
                Override::Set(attrs.font_variations.clone())
            },
            text_decoration: if attrs.text_decoration == defaults.text_decoration {
                Override::Inherit
            } else {
                Override::Set(attrs.text_decoration)
            },
            optical_size: if attrs.optical_size == defaults.optical_size {
                Override::Inherit
            } else {
                Override::Set(attrs.optical_size)
            },
        }
    }

    /// Returns `true` if every field is [`Override::Inherit`] — i.e.
    /// the override has no effect against any defaults.
    pub fn is_empty(&self) -> bool {
        self.color.is_inherit()
            && self.metrics.is_inherit()
            && self.letter_spacing.is_inherit()
            && self.family.is_inherit()
            && self.stretch.is_inherit()
            && self.style.is_inherit()
            && self.weight.is_inherit()
            && self.metadata.is_inherit()
            && self.cache_key_flags.is_inherit()
            && self.font_features.is_inherit()
            && self.font_variations.is_inherit()
            && self.text_decoration.is_inherit()
            && self.optical_size.is_inherit()
    }

    /// Merge this override on top of `defaults`, producing a complete
    /// [`Attrs`].
    ///
    /// Fields set in `self` use their contained value; fields marked
    /// [`Override::Inherit`] take the corresponding value from
    /// `defaults`.
    pub fn merge<'a>(&'a self, defaults: &'a AttrsOwned) -> Attrs<'a> {
        Attrs {
            color_opt: self.color.resolve_or(defaults.color_opt),
            metrics_opt: self.metrics.resolve_or(defaults.metrics_opt),
            letter_spacing_opt: self.letter_spacing.resolve_or(defaults.letter_spacing_opt),
            stretch: self.stretch.resolve_or(defaults.stretch),
            style: self.style.resolve_or(defaults.style),
            weight: self.weight.resolve_or(defaults.weight),
            metadata: self.metadata.resolve_or(defaults.metadata),
            cache_key_flags: self.cache_key_flags.resolve_or(defaults.cache_key_flags),
            text_decoration: self.text_decoration.resolve_or(defaults.text_decoration),
            optical_size: self.optical_size.resolve_or(defaults.optical_size),
            family: match &self.family {
                Override::Inherit => defaults.family_owned.as_family(),
                Override::Set(f) => f.as_family(),
            },
            font_features: match &self.font_features {
                Override::Inherit => defaults.font_features.clone(),
                Override::Set(f) => f.clone(),
            },
            font_variations: match &self.font_variations {
                Override::Inherit => defaults.font_variations.clone(),
                Override::Set(v) => v.clone(),
            },
        }
    }
}

/// List of text attributes to apply to a line
//TODO: have this clean up the spans when changes are made
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AttrsList {
    pub(crate) defaults: AttrsOwned,
    pub(crate) spans: RangeMap<usize, AttrsOverride>,
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

    /// Get the current attribute spans
    pub fn spans(&self) -> Vec<(&Range<usize>, &AttrsOverride)> {
        self.spans_iter().collect()
    }

    /// Get an iterator over the current attribute spans
    pub fn spans_iter(&self) -> impl Iterator<Item = (&Range<usize>, &AttrsOverride)> + '_ {
        self.spans.iter()
    }

    /// Clear the current attribute spans
    pub fn clear_spans(&mut self) {
        self.spans.clear();
    }

    /// Add a sparse override span, removes any previous matching parts of
    /// spans.
    ///
    /// Each field of `over` is either `Inherit` (resolves to the
    /// corresponding default at lookup time via [`Self::get_span`]) or
    /// `Set(v)` (overrides the default for that range). The override is
    /// stored as-is — changing line defaults later naturally re-inherits
    /// for all `Inherit` fields, with no rewrite of stored spans needed.
    pub fn add_span(&mut self, range: Range<usize>, over: &AttrsOverride) {
        //do not support 1..1 or 2..1 even if by accident.
        if range.is_empty() {
            return;
        }

        self.spans.insert(range, over.clone());
    }

    /// Add an attribute span from a full [`Attrs`] value, computing the
    /// sparse override against the current defaults.
    ///
    /// Backwards-compat helper for callers that build full `Attrs` and
    /// hand them off. New code should construct an [`AttrsOverride`]
    /// directly and call [`Self::add_span`].
    pub fn add_span_from_attrs(&mut self, range: Range<usize>, attrs: &Attrs) {
        if range.is_empty() {
            return;
        }
        let over = AttrsOverride::diff(&self.defaults.as_attrs(), attrs);
        self.spans.insert(range, over);
    }

    /// Get the resolved attributes at `index`, merging the per-position
    /// sparse override (if any) on top of the line defaults.
    pub fn get_span(&self, index: usize) -> Attrs<'_> {
        match self.spans.get(&index) {
            None => self.defaults.as_attrs(),
            Some(over) => over.merge(&self.defaults),
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
            let (range, over) = self
                .spans
                .get_key_value(&key.start)
                .map(|v| (v.0.clone(), v.1.clone()))
                .expect("attrs span not found");
            self.spans.remove(key);

            if resize {
                new.spans.insert(0..range.end - index, over.clone());
                self.spans.insert(range.start..index, over);
            } else {
                new.spans
                    .insert(range.start - index..range.end - index, over);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_default_is_inherit() {
        let o: Override<Weight> = Override::default();
        assert!(o.is_inherit());
        assert!(!o.is_set());
    }

    #[test]
    fn override_resolve_or_falls_back_when_inherit() {
        let o: Override<Weight> = Override::Inherit;
        assert_eq!(o.resolve_or(Weight::BOLD), Weight::BOLD);
    }

    #[test]
    fn override_resolve_or_returns_set_value() {
        let o: Override<Weight> = Override::Set(Weight::NORMAL);
        assert_eq!(o.resolve_or(Weight::BOLD), Weight::NORMAL);
    }

    #[test]
    fn override_as_ref_borrows_set_value() {
        let o: Override<FamilyOwned> = Override::Set(FamilyOwned::SansSerif);
        match o.as_ref() {
            Override::Set(f) => assert_eq!(*f, FamilyOwned::SansSerif),
            Override::Inherit => panic!("expected Set"),
        }
    }

    #[test]
    fn attrs_override_default_is_all_inherit() {
        let o = AttrsOverride::default();
        assert!(o.is_empty());
    }

    #[test]
    fn diff_against_equal_attrs_is_empty() {
        let a = Attrs::new();
        let o = AttrsOverride::diff(&a, &a);
        assert!(o.is_empty());
    }

    #[test]
    fn diff_against_different_weight_sets_weight_field() {
        let defaults = Attrs::new();
        let bold = Attrs::new().weight(Weight::BOLD);
        let o = AttrsOverride::diff(&defaults, &bold);
        assert!(!o.is_empty());
        assert_eq!(o.weight, Override::Set(Weight::BOLD));
        // Other fields remain Inherit.
        assert!(o.color.is_inherit());
        assert!(o.style.is_inherit());
        assert!(o.metrics.is_inherit());
    }

    #[test]
    fn diff_round_trip_preserves_semantic_equality() {
        let defaults_owned = AttrsOwned::new(&Attrs::new());
        let attrs = Attrs::new()
            .weight(Weight::BOLD)
            .style(Style::Italic)
            .metrics(Metrics::new(20.0, 24.0));

        let over = AttrsOverride::diff(&defaults_owned.as_attrs(), &attrs);
        let merged = over.merge(&defaults_owned);

        assert_eq!(merged.weight, attrs.weight);
        assert_eq!(merged.style, attrs.style);
        assert_eq!(merged.metrics_opt, attrs.metrics_opt);
        // Untouched fields equal defaults.
        assert_eq!(merged.color_opt, defaults_owned.color_opt);
        assert_eq!(merged.family, defaults_owned.family_owned.as_family());
    }

    #[test]
    fn set_span_style_then_change_defaults_inherits_new_size() {
        // Bug B reproduction at the cosmic-text level: when a sparse
        // override on a span has `Inherit` for size and bold, swapping
        // the line's default attrs (e.g., demoting heading → body)
        // must let the span resolve against the *new* defaults instead
        // of carrying the old size/bold forward.

        // Heading-like defaults: bold + size 32.
        let heading_attrs = Attrs::new()
            .weight(Weight::BOLD)
            .metrics(Metrics::new(32.0, 38.0));
        let mut heading_list = AttrsList::new(&heading_attrs);

        // Add a span that explicitly sets *only* italic — bold and
        // size stay `Inherit`.
        let italic_over = AttrsOverride {
            style: Override::Set(Style::Italic),
            ..Default::default()
        };
        heading_list.add_span(0..5, &italic_over);

        // Under heading defaults: merged Attrs picks up bold + size 32
        // from defaults, italic from the override.
        let resolved = heading_list.get_span(0);
        assert_eq!(resolved.weight, Weight::BOLD);
        assert_eq!(resolved.style, Style::Italic);
        assert!(resolved.metrics_opt.is_some());

        // Swap to body defaults (no bold, no metrics override) and
        // re-add the *same* override — mimicking what an editor does
        // when paragraph style changes.
        let body_attrs = Attrs::new();
        let mut body_list = AttrsList::new(&body_attrs);
        body_list.add_span(0..5, &italic_over);

        // The override is unchanged — but now it resolves against
        // body defaults, so bold and size disappear.
        let resolved = body_list.get_span(0);
        assert_eq!(resolved.weight, Weight::NORMAL);
        assert_eq!(resolved.style, Style::Italic);
        assert_eq!(resolved.metrics_opt, None);
    }

    #[test]
    fn set_none_distinct_from_inherit_for_color() {
        // Defaults carry a color; a span wants to explicitly clear it.
        let defaults_attrs = Attrs::new().color(Color(0xff_00_00_00));
        let defaults_owned = AttrsOwned::new(&defaults_attrs);

        // Inherit → merged color is defaults' color.
        let inherit_over = AttrsOverride::default();
        assert_eq!(
            inherit_over.merge(&defaults_owned).color_opt,
            Some(Color(0xff_00_00_00))
        );

        // Set(None) → merged color is explicitly cleared.
        let clear_over = AttrsOverride {
            color: Override::Set(None),
            ..Default::default()
        };
        assert_eq!(clear_over.merge(&defaults_owned).color_opt, None);

        // Set(Some(c)) → merged color is c.
        let set_over = AttrsOverride {
            color: Override::Set(Some(Color(0x00_00_ff_ff))),
            ..Default::default()
        };
        assert_eq!(
            set_over.merge(&defaults_owned).color_opt,
            Some(Color(0x00_00_ff_ff))
        );
    }
}
