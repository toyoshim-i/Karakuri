use serde::{Deserialize, Serialize};

/// A typed identifier for a split in a layout arrangement.
///
/// Unlike view leaves which always require a string name, structural splits
/// may be anonymous in the user manual (such as the root column or body row)
/// or named (such as `"left-pane"` or `"centre"`). `LayoutSplit` gives typed
/// identity to both categories so they can be addressed and operated without
/// relying on arbitrary string hacks or unrepresentable anonymous splits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayoutSplit {
    /// The root column of the arrangement.
    RootColumn,
    /// The body row holding the main panes.
    BodyRow,
    /// An explicitly named split.
    Named(String),
}

impl LayoutSplit {
    /// The string identifier or canonical representation.
    pub fn as_str(&self) -> &str {
        match self {
            LayoutSplit::RootColumn => "root-column",
            LayoutSplit::BodyRow => "body-row",
            LayoutSplit::Named(s) => s.as_str(),
        }
    }
}

impl std::fmt::Display for LayoutSplit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for LayoutSplit {
    fn from(s: &str) -> Self {
        match s {
            "root-column" => LayoutSplit::RootColumn,
            "body-row" => LayoutSplit::BodyRow,
            other => LayoutSplit::Named(other.to_owned()),
        }
    }
}

impl From<String> for LayoutSplit {
    fn from(s: String) -> Self {
        match s.as_str() {
            "root-column" => LayoutSplit::RootColumn,
            "body-row" => LayoutSplit::BodyRow,
            _ => LayoutSplit::Named(s),
        }
    }
}
