//! Node addresses, ports, revisions, and parameter identifiers.

use super::layer::Layer;

/// Placeholder for operation payloads whose specification is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Undecided;

/// Name of a procedure input port (`InputPort`) declared in an interface.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputPort(pub String);

impl InputPort {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for InputPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for InputPort {
    fn from(name: String) -> InputPort {
        InputPort(name)
    }
}

impl From<&str> for InputPort {
    fn from(name: &str) -> InputPort {
        InputPort(name.to_string())
    }
}

/// Positional address of a node within a deck's Set (`{layer, index}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeAddress {
    pub layer: Layer,
    /// Node index within that layer in declaration order.
    pub index: u32,
}

/// Target version for procedure restoration ([`Operation::RestoreProcedure`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision {
    /// Restore the immediately previous version replaced by this node.
    Previous(NodeAddress),
    /// Restore an explicit version identified by snapshot name.
    Picked(String),
}

/// Target parameter address within a deck's Set.
///
/// If `node` is `None`, targets all nodes in the Set declaring `key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAt {
    pub node: Option<NodeAddress>,
    /// Parameter identifier within target node(s).
    pub key: String,
}

/// Target parameter address for signal binding attachments.
///
/// Binds to a specific layer and parameter key, optionally restricted to node `index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindAt {
    pub layer: Layer,
    /// Target node index within the layer, or `None` for all nodes declaring `key`.
    pub index: Option<u32>,
    /// Component parameter key (e.g. `glow.x` for vector components).
    pub key: String,
}

/// A parameter's value. The three widths a `.kir` can declare, on
/// `karakuri_store::record::Value`'s terms — a value and never a range, since a
/// range is the procedure's declaration and not an operator's to write.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParamValue {
    Scalar(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
}
