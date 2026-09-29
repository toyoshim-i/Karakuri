//! Presets and content addresses for shipped master chain procedures (ADR-0340).

use std::sync::OnceLock;

/// `examples/feedback.kir` — the one of the three that declares `retains`.
pub const FEEDBACK: &str = include_str!("../../../../examples/feedback.kir");
/// `examples/bloom.kir` — one 9x9 pass where the hand-written form was two.
pub const BLOOM: &str = include_str!("../../../../examples/bloom.kir");
/// `examples/rgb_shift.kir`.
pub const RGB_SHIFT: &str = include_str!("../../../../examples/rgb_shift.kir");
/// `examples/analog_tv.kir`.
pub const ANALOG_TV: &str = include_str!("../../../../examples/analog_tv.kir");
/// `examples/crt_screen.kir`.
pub const CRT_SCREEN: &str = include_str!("../../../../examples/crt_screen.kir");
/// `examples/chroma_echo.kir`.
pub const CHROMA_ECHO: &str = include_str!("../../../../examples/chroma_echo.kir");
/// `examples/chroma_burst.kir`.
pub const CHROMA_BURST: &str = include_str!("../../../../examples/chroma_burst.kir");
/// `examples/film_grain.kir`.
pub const FILM_GRAIN: &str = include_str!("../../../../examples/film_grain.kir");
/// `examples/glitch_slice.kir`.
pub const GLITCH_SLICE: &str = include_str!("../../../../examples/glitch_slice.kir");
/// `examples/negative_strobe.kir`.
pub const NEGATIVE_STROBE: &str = include_str!("../../../../examples/negative_strobe.kir");
/// `examples/slit_scan.kir`.
pub const SLIT_SCAN: &str = include_str!("../../../../examples/slit_scan.kir");
/// `examples/acrylic_glow.kir`.
pub const ACRYLIC_GLOW: &str = include_str!("../../../../examples/acrylic_glow.kir");
/// `examples/beat_dissolve.kir`.
pub const BEAT_DISSOLVE: &str = include_str!("../../../../examples/beat_dissolve.kir");
/// `examples/cathode_bend.kir`.
pub const CATHODE_BEND: &str = include_str!("../../../../examples/cathode_bend.kir");
/// `examples/flash_panels.kir`.
pub const FLASH_PANELS: &str = include_str!("../../../../examples/flash_panels.kir");
/// `examples/hue_rotate.kir`.
pub const HUE_ROTATE: &str = include_str!("../../../../examples/hue_rotate.kir");
/// `examples/led_matrix.kir`.
pub const LED_MATRIX: &str = include_str!("../../../../examples/led_matrix.kir");
/// `examples/lens_warp.kir`.
pub const LENS_WARP: &str = include_str!("../../../../examples/lens_warp.kir");
/// `examples/mirror_fold.kir`.
pub const MIRROR_FOLD: &str = include_str!("../../../../examples/mirror_fold.kir");
/// `examples/mosaic_glow.kir`.
pub const MOSAIC_GLOW: &str = include_str!("../../../../examples/mosaic_glow.kir");
/// `examples/neon_edges.kir`.
pub const NEON_EDGES: &str = include_str!("../../../../examples/neon_edges.kir");
/// `examples/poster_dither.kir`.
pub const POSTER_DITHER: &str = include_str!("../../../../examples/poster_dither.kir");
/// `examples/roll_panels.kir`.
pub const ROLL_PANELS: &str = include_str!("../../../../examples/roll_panels.kir");
/// `examples/sepia_film.kir`.
pub const SEPIA_FILM: &str = include_str!("../../../../examples/sepia_film.kir");
/// `examples/shadow_mask.kir`.
pub const SHADOW_MASK: &str = include_str!("../../../../examples/shadow_mask.kir");
/// `examples/tile_zoom.kir`.
pub const TILE_ZOOM: &str = include_str!("../../../../examples/tile_zoom.kir");
/// `examples/tuning_moire.kir`.
pub const TUNING_MOIRE: &str = include_str!("../../../../examples/tuning_moire.kir");
/// `examples/wired_static.kir`.
pub const WIRED_STATIC: &str = include_str!("../../../../examples/wired_static.kir");
/// `examples/zoom_blur.kir`.
pub const ZOOM_BLUR: &str = include_str!("../../../../examples/zoom_blur.kir");

/// All shipped master chain procedures in canonical order.
pub const ALL: [(&str, &str); 29] = [
    ("feedback", FEEDBACK),
    ("bloom", BLOOM),
    ("rgb_shift", RGB_SHIFT),
    ("analog_tv", ANALOG_TV),
    ("crt_screen", CRT_SCREEN),
    ("chroma_echo", CHROMA_ECHO),
    ("chroma_burst", CHROMA_BURST),
    ("film_grain", FILM_GRAIN),
    ("glitch_slice", GLITCH_SLICE),
    ("negative_strobe", NEGATIVE_STROBE),
    ("slit_scan", SLIT_SCAN),
    ("acrylic_glow", ACRYLIC_GLOW),
    ("beat_dissolve", BEAT_DISSOLVE),
    ("cathode_bend", CATHODE_BEND),
    ("flash_panels", FLASH_PANELS),
    ("hue_rotate", HUE_ROTATE),
    ("led_matrix", LED_MATRIX),
    ("lens_warp", LENS_WARP),
    ("mirror_fold", MIRROR_FOLD),
    ("mosaic_glow", MOSAIC_GLOW),
    ("neon_edges", NEON_EDGES),
    ("poster_dither", POSTER_DITHER),
    ("roll_panels", ROLL_PANELS),
    ("sepia_film", SEPIA_FILM),
    ("shadow_mask", SHADOW_MASK),
    ("tile_zoom", TILE_ZOOM),
    ("tuning_moire", TUNING_MOIRE),
    ("wired_static", WIRED_STATIC),
    ("zoom_blur", ZOOM_BLUR),
];

/// A curated master chain preset configuration (M10.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainPreset {
    pub name: &'static str,
    pub description: &'static str,
    pub procedures: &'static [&'static str],
}

impl ChainPreset {
    /// Builds a [`karakuri_store::record::Chain`] record representation of this preset.
    pub fn build_chain(&self) -> karakuri_store::record::Chain {
        let mut slots = Vec::new();
        for &name in self.procedures {
            if let Some((_, src)) = ALL.iter().find(|(n, _)| *n == name) {
                let retains = matches!(name, "feedback" | "chroma_echo" | "slit_scan");
                slots.push(karakuri_store::record::ChainSlot {
                    procedure: address(src),
                    cut: if retains {
                        Some("mix".to_string())
                    } else {
                        None
                    },
                    params: std::collections::BTreeMap::new(),
                });
            }
        }
        karakuri_store::record::Chain { slots }
    }
}

pub const CLEAN_CYBER: ChainPreset = ChainPreset {
    name: "Clean Cyber",
    description: "High-clarity bloom with chromatic burst on transients",
    procedures: &["bloom", "chroma_burst"],
};

pub const RETRO_STAGE: ChainPreset = ChainPreset {
    name: "Retro Stage",
    description: "Cathode display phosphor pattern with analog distortion and film grain",
    procedures: &["crt_screen", "analog_tv", "film_grain"],
};

pub const PSYCHEDELIC_ECHO: ChainPreset = ChainPreset {
    name: "Psychedelic Echo",
    description: "Deep video feedback loop with chromatic trails and slit-scan warp",
    procedures: &["feedback", "chroma_echo", "slit_scan"],
};

pub const DROP_ASSAULT: ChainPreset = ChainPreset {
    name: "Drop Assault",
    description: "High-intensity glitch slice displacement, negative strobe pulse, and bloom",
    procedures: &["glitch_slice", "negative_strobe", "bloom"],
};

/// Curated master chain presets saved to and recalled from the Library (M10.3).
pub const CHAIN_PRESETS: [&ChainPreset; 4] =
    [&CLEAN_CYBER, &RETRO_STAGE, &PSYCHEDELIC_ECHO, &DROP_ASSAULT];

/// The content address of one shipped source, spelled the way a record spells
/// one.
pub fn address(source: &str) -> String {
    // `Display` already writes the `sha256:` prefix — see
    // `karakuri_store::hash::Hash`, whose `FromStr` requires it.
    karakuri_store::hash::Hash::of(source.as_bytes()).to_string()
}

/// The content addresses of the three, in the order the Master bay draws them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Shipped {
    pub feedback: String,
    pub bloom: String,
    pub rgb_shift: String,
}

impl Shipped {
    /// The three, in [`ALL`]'s order, so a caller can walk them beside the bay's
    /// rows.
    pub fn each(&self) -> [&str; 3] {
        [&self.feedback, &self.bloom, &self.rgb_shift]
    }
}

/// The three addresses, computed once. Hashing three files is a few
/// microseconds and it is still done once, because this is asked per press.
pub fn addresses() -> &'static Shipped {
    static ONCE: OnceLock<Shipped> = OnceLock::new();
    ONCE.get_or_init(|| Shipped {
        feedback: address(FEEDBACK),
        bloom: address(BLOOM),
        rgb_shift: address(RGB_SHIFT),
    })
}

/// The word the Master bay draws for one of the shipped procedures, where the address is one
/// of them.
pub fn name_of(address_of: &str) -> Option<&'static str> {
    for (name, source) in ALL {
        if name == address_of || address(source) == address_of {
            return Some(match name {
                "acrylic_glow" => "acrylic glow",
                "analog_tv" => "analog tv",
                "beat_dissolve" => "beat dissolve",
                "cathode_bend" => "cathode bend",
                "chroma_burst" => "chroma burst",
                "chroma_echo" => "chroma echo",
                "crt_screen" => "crt screen",
                "film_grain" => "film grain",
                "flash_panels" => "flash panels",
                "glitch_slice" => "glitch slice",
                "hue_rotate" => "hue rotate",
                "led_matrix" => "led matrix",
                "lens_warp" => "lens warp",
                "mirror_fold" => "mirror fold",
                "mosaic_glow" => "mosaic glow",
                "negative_strobe" => "negative strobe",
                "neon_edges" => "neon edges",
                "poster_dither" => "poster dither",
                "rgb_shift" => "rgb shift",
                "roll_panels" => "roll panels",
                "sepia_film" => "sepia film",
                "shadow_mask" => "shadow mask",
                "slit_scan" => "slit scan",
                "tile_zoom" => "tile zoom",
                "tuning_moire" => "tuning moire",
                "wired_static" => "wired static",
                "zoom_blur" => "zoom blur",
                other => other,
            });
        }
    }
    None
}

/// Returns the source code for a shipped preset address or procedure name, or `None` if unknown.
pub fn source(address_of: &str) -> Option<&'static str> {
    ALL.into_iter()
        .find(|&(name, src)| name == address_of || address(src) == address_of)
        .map(|(_, src)| src)
}
