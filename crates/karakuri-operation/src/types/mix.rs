//! Mixing, compositing, tone mapping, and transition parameters.

/// Destination target for composited video output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Output {
    /// Program bay preview output.
    Program,
    /// Windowed projector display output indexed by window instance.
    Projector(u8),
    /// External sink provided by an output plugin (e.g. Syphon, NDI).
    Plugin(u8),
}

impl Output {
    /// Standard built-in output targets.
    pub const ALL: [Output; 2] = [Output::Program, Output::Projector(0)];

    /// Returns the human-readable identifier for this output target.
    pub fn name(self) -> String {
        match self {
            Output::Program => "program view".to_owned(),
            Output::Projector(n) => format!("projector {n}"),
            Output::Plugin(n) => format!("plugin {n}"),
        }
    }
}

/// Layer compositing blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    Add,
    Over,
    Max,
}

impl BlendMode {
    /// All available blend modes in canonical cycle order.
    pub const ALL: [BlendMode; 3] = [BlendMode::Add, BlendMode::Over, BlendMode::Max];

    /// Returns the canonical lowercase identifier for this blend mode.
    pub fn name(self) -> &'static str {
        match self {
            BlendMode::Add => "add",
            BlendMode::Over => "over",
            BlendMode::Max => "max",
        }
    }
}

/// The transfer from unbounded linear HDR to something displayable.
/// `karakuri_engine::present::TonemapOp`'s four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tonemap {
    Clamp,
    Reinhard,
    Aces,
    AgX,
}

impl Tonemap {
    /// Returns the canonical lowercase identifier for this tonemap operator.
    pub fn name(self) -> &'static str {
        match self {
            Tonemap::Clamp => "clamp",
            Tonemap::Reinhard => "reinhard",
            Tonemap::Aces => "aces",
            Tonemap::AgX => "agx",
        }
    }
}

/// Which frame the master chain feedback pass reads back.
///
/// Mirrors `karakuri_engine::master::Cut`. See ADR-0317.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cut {
    /// The frame as the mix wrote it, before this chain touched it.
    #[default]
    Mix,
    /// The chain's own exit, after rgb shift and before tone mapping.
    Exit,
}

impl Cut {
    /// Both cut sources in display order.
    pub const ALL: [Cut; 2] = [Cut::Mix, Cut::Exit];

    /// Returns the canonical lowercase identifier for this cut.
    pub fn name(self) -> &'static str {
        match self {
            Cut::Mix => "mix",
            Cut::Exit => "exit",
        }
    }
}

/// Master chain feedback parameters specifying recirculation amount and tap point.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Feedback {
    /// Feedback gain in `[0.0, 0.95]`.
    pub amount: f32,
    /// Feedback tap source frame.
    pub cut: Cut,
}

impl Feedback {
    /// Maximum feedback amount allowed (0.95) to prevent unbounded accumulation under [`Cut::Exit`].
    pub const MAX: f32 = 0.95;
}

/// Master chain slot parameter configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum ChainParam {
    /// One parameter the slot's procedure declares, set outright.
    Declared {
        /// Parameter key name.
        key: String,
        /// Parameter value.
        value: f32,
    },
    /// Frame feedback cut source for slots declaring `retains`.
    Cut(Cut),
}

/// How a signal is shaped on its way to a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Curve {
    Lin,
    Pow2,
    Sqrt,
    Smooth,
}

impl Curve {
    /// All available signal shaping transfer curves.
    pub const ALL: [Curve; 4] = [Curve::Lin, Curve::Pow2, Curve::Sqrt, Curve::Smooth];

    /// Returns the canonical lowercase identifier for this transfer curve.
    pub fn name(self) -> &'static str {
        match self {
            Curve::Lin => "lin",
            Curve::Pow2 => "pow2",
            Curve::Sqrt => "sqrt",
            Curve::Smooth => "smooth",
        }
    }
}

/// Geometric shape of a transition wipe boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeKind {
    /// No mask applied.
    None,
    /// Linear planar front oriented at an angle.
    Linear,
    /// Radial circular iris expanding from the frame center.
    Radial,
}

impl WipeKind {
    /// Returns the canonical lowercase identifier for this wipe shape.
    pub fn name(self) -> &'static str {
        match self {
            WipeKind::None => "none",
            WipeKind::Linear => "linear",
            WipeKind::Radial => "radial",
        }
    }
}

/// Configurable parameter for crossfade and wipe transitions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionSetting {
    /// Geometric wipe shape and front angle in radians.
    WipeShape { kind: WipeKind, angle: f32 },
    /// Metric grid alignment for transition start in beats.
    Quantum { beats: f64 },
    /// Transition duration in beats (0.0 represents an instantaneous cut).
    Length { beats: f64 },
}
