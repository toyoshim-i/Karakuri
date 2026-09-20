use karakuri_operation::Layer;

use super::super::filters::LAYERS;

// ---------------------------------------------------------------------------
// Library listing rows and data models
// ---------------------------------------------------------------------------

/// A library row taken in hand: which row, and the Set on it.
#[derive(Debug, Clone, PartialEq)]
pub struct Taken {
    /// Which row of the listing, counting from the top of the drawn list.
    pub row: usize,
    /// The Set on it, by the name the listing carried.
    pub set: String,
    /// Whether that name is a procedure's — which decides whether a drop over a
    /// strip names `Operation::LoadProcedure` or `Operation::LoadSet` (ADR-0338).
    pub procedure: bool,
}

/// What one row of the Library bay's listing is: a Set or a procedure, and the
/// layers its badge names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowKind {
    /// The layers this row implements, in the order they are drawn — a Set's slots,
    /// or a procedure's one `kind`.
    pub badges: Vec<Layer>,
    /// A procedure and not a Set.
    pub procedure: bool,
}

/// The listing, as the controls of this bay read it: the names and what each
/// row is.
#[derive(Debug, Clone, Copy)]
pub struct Rows<'a> {
    /// The names the bay lists, in the order the host listed them —
    /// [`View::library`].
    pub names: &'a [String],
    /// What each of those rows is — [`View::kinds`], in the same order.
    pub kinds: &'a [RowKind],
}

impl<'a> Rows<'a> {
    /// Nothing listed at all, which is a scope whose rows this control cannot name
    /// — `history` under [`View::rows`], whose rows are versions.
    pub const NONE: Rows<'static> = Rows {
        names: &[],
        kinds: &[],
    };

    /// How many rows there are, which is the names': the kinds are a decoration of
    /// them and a shorter row of kinds lists nothing away.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the listing holds nothing.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The name on this row, or `None` past the end.
    pub fn name(&self, row: usize) -> Option<&'a str> {
        self.names.get(row).map(String::as_str)
    }

    /// Whether this row is a procedure.
    pub fn procedure(&self, row: usize) -> bool {
        self.kinds.get(row).is_some_and(|kind| kind.procedure)
    }

    /// The name on this row where it is a Set, and `None` where it is a procedure
    /// or past the end.
    pub fn set(&self, row: usize) -> Option<&'a str> {
        (!self.procedure(row)).then(|| self.name(row)).flatten()
    }

    /// The words on this row's badges, in the order they are drawn — [`LAYERS`]'
    /// spelling.
    pub fn badges(&self, row: usize) -> Vec<&'static str> {
        self.kinds
            .get(row)
            .map(|kind| {
                kind.badges
                    .iter()
                    .filter_map(|layer| {
                        LAYERS
                            .iter()
                            .find(|(kind, _)| kind == layer)
                            .map(|(_, word)| *word)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Where this library is pointed: the directory in the `.path` row, and whether
/// what it reads is a folder that has *arrived* or one that is on its way in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointed<'a> {
    /// The directory, as the host spells it.
    pub path: &'a str,
    /// A folder is over the window and this is what a release would set.
    pub incoming: bool,
}
