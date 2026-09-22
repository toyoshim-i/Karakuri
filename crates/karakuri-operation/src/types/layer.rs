//! Layer definitions and parsers for Karakuri Set layers.

/// Which layer of a Set a node sits on.
///
/// `Field` addresses no node in the rendering sense — it has no pass and no
/// buffers, and lowers into whoever evaluates it — but its params are declared
/// and addressable, which is why it is here at all. That paragraph is
/// `karakuri_store::record::Layer`'s, and this is a third spelling of that list
/// beside `karakuri_ir::ast::Kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    L1,
    L2,
    L3,
    L4,
    Field,
    /// A `kind L5` procedure — a frame effect over the picture handed to it. See
    /// `docs/adr/0340-kind-l5-is-written-and-the-master-chain-is-an-ordered-list-of-them.md`.
    L5,
}

impl Layer {
    /// Every layer variant, in canonical order.
    pub const ALL: [Layer; 6] = [
        Layer::L1,
        Layer::L2,
        Layer::L3,
        Layer::L4,
        Layer::Field,
        Layer::L5,
    ];

    /// Name of the layer as used across configurations and displays.
    pub const fn name(&self) -> &'static str {
        match self {
            Layer::L1 => "L1",
            Layer::L2 => "L2",
            Layer::L3 => "L3",
            Layer::L4 => "L4",
            Layer::Field => "Field",
            Layer::L5 => "L5",
        }
    }
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// An error returned when parsing a [`Layer`] from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseLayerError(String);

impl std::fmt::Display for ParseLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown layer: `{}`; expected one of L1, L2, L3, L4, Field, L5",
            self.0
        )
    }
}

impl std::error::Error for ParseLayerError {}

impl std::str::FromStr for Layer {
    type Err = ParseLayerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "L1" => Ok(Layer::L1),
            "L2" => Ok(Layer::L2),
            "L3" => Ok(Layer::L3),
            "L4" => Ok(Layer::L4),
            "FIELD" => Ok(Layer::Field),
            "L5" => Ok(Layer::L5),
            _ => Err(ParseLayerError(s.to_string())),
        }
    }
}
