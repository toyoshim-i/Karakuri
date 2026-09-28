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

/// All shipped master chain procedures in canonical order.
pub const ALL: [(&str, &str); 11] = [
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
        if address(source) == address_of {
            return Some(match name {
                "rgb_shift" => "rgb shift",
                "analog_tv" => "analog tv",
                "crt_screen" => "crt screen",
                "chroma_echo" => "chroma echo",
                "chroma_burst" => "chroma burst",
                "film_grain" => "film grain",
                "glitch_slice" => "glitch slice",
                "negative_strobe" => "negative strobe",
                "slit_scan" => "slit scan",
                other => other,
            });
        }
    }
    None
}

/// Returns the source code for a shipped preset address, or `None` if unknown.
pub fn source(address_of: &str) -> Option<&'static str> {
    ALL.into_iter()
        .map(|(_, src)| src)
        .find(|src| address(src) == address_of)
}
