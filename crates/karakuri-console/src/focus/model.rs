use super::*;

/// The digit (`0`) that names a bay's head rather than an item.
pub const HEAD: usize = 0;

/// Name of the Mixer bay, whose remembered item tracks deck selection.
pub const MIXER: &str = "mixer";

/// The Library bay — whose remembered item is the row under the cursor and
/// whose head's remembered control is the marked scope. [`MIXER`]'s reason.
pub const LIBRARY: &str = "library";

/// Returns whether a region is one of the nine top-level focusable bays (ADR-0259).
///
/// Sub-regions within Program and Inspector are treated as items reached via digits rather than `Tab`.
pub const fn is_bay(kind: Kind) -> bool {
    match kind {
        Kind::Bay { .. }
        | Kind::Transport
        | Kind::Library
        | Kind::Staging
        | Kind::Mixer
        | Kind::Master
        | Kind::Sequencer
        | Kind::Outputs => true,
        Kind::Picture | Kind::Previews | Kind::Pane => false,
    }
}

/// The tab ring: all bay regions of `layout` in column-major visual order.
pub fn ring(layout: &Layout) -> Vec<&'static Region> {
    let mut bays = Vec::new();
    descend(layout, layout.root(), &mut bays);
    bays
}

/// [`ring`]'s walk, which stops at a bay rather than descending into it.
fn descend(layout: &Layout, id: NodeId, bays: &mut Vec<&'static Region>) {
    if let Some(found) = layout.name(id).and_then(region) {
        if is_bay(found.kind) {
            bays.push(found);
            return;
        }
    }
    for child in layout.children(id) {
        descend(layout, *child, bays);
    }
}

/// Finds the visible or folded bay at point `p`, excluding divider grab margins to prevent resize gestures from stealing focus.
pub fn bay_at(layout: &Layout, p: Point) -> Option<&'static Region> {
    if matches!(
        layout.hit(p, crate::panel::GRAB),
        karakuri_layout::Hit::Divider { .. }
    ) {
        return None;
    }
    for bay in ring(layout) {
        if let Some(id) = layout.find(bay.name) {
            let rect = layout.rect(id);
            if (layout.visible(id) || (layout.is_collapsed(id) && rect.h > 0.0 && rect.w > 0.0))
                && rect.contains(p)
            {
                return Some(bay);
            }
        }
    }
    None
}

/// Calculates the bounding rectangle for a bay's dashed focus ring, or `None` if folded/hidden (ADR-0159, ADR-0259).
///
/// Covers the bay head, or the entire row for headless bays (Transport, Outputs). Requires solved `layout`.
pub fn mark(layout: &Layout, bay: &'static Region) -> Option<egui::Rect> {
    let id = layout.find(bay.name)?;
    if !layout.visible(id) {
        return None;
    }
    let rect = crate::view::to_egui(layout.rect(id));
    Some(match crate::view::head_of(bay) {
        Some(_) => crate::view::head_box(rect),
        None => rect,
    })
}

/// Calculates the focus mark rectangle for a collapsed/folded bay (ADR-0259).
///
/// Projects the residual edge of the collapsed child into a head-height indicator. Requires solved `layout`.
pub fn folded_head(layout: &Layout, bay: &'static Region) -> Option<egui::Rect> {
    let id = layout.find(bay.name)?;
    if layout.visible(id) || !layout.is_collapsed(id) {
        return None;
    }
    let parent = layout.parent(id)?;
    if !layout.visible(parent) {
        return None;
    }
    let edge = crate::view::to_egui(layout.rect(id));
    if edge.width() > 0.0 && edge.height() > 0.0 {
        return Some(edge);
    }
    let inside = crate::view::to_egui(layout.rect(parent));
    let width = match edge.width() > 1.0 {
        true => edge.width(),
        false => inside.width(),
    };
    let head = egui::Rect::from_min_size(edge.min, egui::vec2(width, crate::room::size::HEAD_H));
    if head.height() > inside.height() || head.width() > inside.width() {
        return None;
    }
    Some(head.translate(egui::vec2(
        (inside.max.x - head.max.x).min(0.0),
        (inside.max.y - head.max.y).min(0.0),
    )))
}

/// The word a folded bay's mark carries beside its title, which is the whole of
/// what it says: there is a bay here, and this is the press that opens it.
/// `console.html` draws exactly this string.
pub const OPENS: &str = "space opens";

/// Pseudo-bay routing key for grammar actions shared across all bays (e.g., `space` to fold).
pub const ANY: &str = "any";

/// Bay address tracking both current focus path and remembered selection per level (ADR-0259).
///
/// Maintains current location (`at`, dashed focus ring) separately from persistent memory (`remembered`, solid ring).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    /// Where the address is now, from the bay down. Empty is the bay itself, which
    /// is where every bay starts and where `esc` stops.
    at: Vec<usize>,
    /// The 1-indexed element last selected under each path prefix (excluding [`HEAD`]).
    memory: BTreeMap<Vec<usize>, usize>,
}

impl Address {
    /// Where the address is now, from the bay down — empty for the bay itself.
    pub fn at(&self) -> &[usize] {
        &self.at
    }

    /// Returns the 1-indexed element last selected under path `under`, or `None`.
    pub fn remembered(&self, under: &[usize]) -> Option<usize> {
        self.memory.get(under).copied()
    }

    /// Records the 1-indexed selection `nth` under `under`, returning `true` if changed (refuses [`HEAD`]).
    pub fn remember(&mut self, under: &[usize], nth: usize) -> bool {
        if nth == HEAD {
            return false;
        }
        self.memory.insert(under.to_vec(), nth) != Some(nth)
    }

    /// Descends focus path to `nth` child below current address and remembers the selection.
    pub fn down(&mut self, nth: usize) {
        self.remember(&self.at.clone(), nth);
        self.at.push(nth);
    }

    /// Ascends one level in the focus address hierarchy, returning true if not already at bay level.
    pub fn up(&mut self) -> bool {
        self.at.pop().is_some()
    }

    /// Resets address to bay root (`[]`) while preserving remembered selections.
    pub fn to_the_bay(&mut self) {
        self.at.clear();
    }

    /// Replaces the current item-level address with `nth` and remembers it.
    pub fn to_item(&mut self, nth: usize) {
        self.at.clear();
        self.down(nth);
    }

    /// Replaces the deepest step of the address with `nth` and remembers it; returns `false` at bay root.
    pub fn to_row(&mut self, nth: usize) -> bool {
        if self.at.pop().is_none() {
            return false;
        }
        self.down(nth);
        true
    }
}

/// Tracks focused bay and remembers internal navigation addresses across bays.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Focus {
    /// Name of currently focused bay, or `None` to default to first traversal bay (ADR-0259).
    at: Option<&'static str>,
    /// Every bay that has ever been addressed, and what it remembers. A bay with no
    /// entry has been addressed by nobody, which is where a run starts.
    addresses: BTreeMap<&'static str, Address>,
}

impl Focus {
    /// Resolves the currently focused bay against `layout`, falling back to first ring bay if unset or removed.
    pub fn bay(&self, layout: &Layout) -> Option<&'static Region> {
        let bays = ring(layout);
        match self.at {
            Some(name) => bays
                .iter()
                .find(|found| found.name == name)
                .or_else(|| bays.first())
                .copied(),
            None => bays.first().copied(),
        }
    }

    /// Sets focus to `bay` if present in `layout`, returning `true` if focus moved.
    pub fn put(&mut self, layout: &Layout, bay: &str) -> bool {
        let Some(found) = ring(layout).into_iter().find(|found| found.name == bay) else {
            return false;
        };
        let was = self.bay(layout).map(|region| region.name);
        self.at = Some(found.name);
        was != Some(found.name)
    }

    /// Steps focus across bays in tree order (`Tab` / `shift-Tab`, wrapping).
    ///
    /// Preserves internal bay addresses; returns `false` if layout has <= 1 bay.
    pub fn tab(&mut self, layout: &Layout, step: i32) -> bool {
        let bays = ring(layout);
        if bays.len() < 2 {
            return false;
        }
        let here = self.bay(layout).map_or(0, |bay| {
            bays.iter()
                .position(|found| found.name == bay.name)
                .unwrap_or(0)
        });
        let len = bays.len() as i32;
        let to = ((here as i32 + step).rem_euclid(len)) as usize;
        let moved = to != here;
        self.at = Some(bays[to].name);
        moved
    }

    /// What `bay` remembers, or `None` for a bay nobody has addressed.
    pub fn address(&self, bay: &str) -> Option<&Address> {
        self.addresses.get(bay)
    }

    /// What `bay` remembers, to write into — created empty on the first write,
    /// which is what makes *a bay nobody has addressed* a state rather than a row
    /// of defaults.
    pub fn address_mut(&mut self, bay: &'static str) -> &mut Address {
        self.addresses.entry(bay).or_default()
    }

    /// Ascends one level up the focused bay's address on `esc`, returning `false` at bay root (ADR-0259).
    pub fn up(&mut self, layout: &Layout) -> bool {
        let Some(bay) = self.bay(layout) else {
            return false;
        };
        match self.addresses.get_mut(bay.name) {
            Some(address) => address.up(),
            None => false,
        }
    }
}

/// Location reached by an address, validated against current bay contents (`None` if stale).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Addressed {
    /// The bay itself, which is where every bay starts and where `esc` stops.
    Bay,
    /// The bay's head — `0`.
    Head,
    /// The nth control of the head.
    OfHead(usize, Control),
    /// A rung under one of the head's controls: the number of the head control the
    /// address descended through, then the number of the thing under it and which
    /// it is. The Sequencer's `+ lane` chooser is the one of these.
    UnderHead {
        through: usize,
        nth: usize,
        control: Control,
    },
    /// Control within a card opened from a headless bay control (e.g. Transport cards).
    InCard {
        through: usize,
        nth: usize,
        control: Control,
    },
    /// The nth item — only in a bay that lists items.
    Item(usize),
    /// A control: the item it is under, `None` in a bay whose items are its
    /// controls, then the control's own number and which it is.
    Of {
        item: Option<usize>,
        nth: usize,
        control: Control,
    },
    /// The third rung: the item, the number of the control the address descended
    /// through, then the number of the thing under it and which it is.
    Under {
        item: usize,
        through: usize,
        nth: usize,
        control: Control,
    },
}

/// Resolves address `at` against bay specification `built` and element counts from `drawn`.
pub fn addressed(
    built: &Built,
    at: &[usize],
    drawn: impl Fn(&[usize]) -> usize,
) -> Option<Addressed> {
    let items = drawn(&[]);
    match *at {
        [] => Some(Addressed::Bay),
        [HEAD] => Some(Addressed::Head),
        [HEAD, nth] => built
            .head
            .get(nth.checked_sub(1)?)
            .copied()
            .map(|control| Addressed::OfHead(nth, control)),
        // Rung under head control (e.g. Sequencer chooser).
        [HEAD, through, nth] => {
            let above = built.head.get(through.checked_sub(1)?).copied()?;
            built
                .beneath(above)
                .nth(nth, drawn(&[HEAD, through]))
                .map(|control| Addressed::UnderHead {
                    through,
                    nth,
                    control,
                })
        }
        // Rung under headless row control (e.g. Transport modal cards).
        [through, nth] if matches!(built.items, Items::Controls(_)) => {
            let above = built.item().nth(through, items)?;
            built
                .beneath(above)
                .nth(nth, drawn(&[through]))
                .map(|control| Addressed::InCard {
                    through,
                    nth,
                    control,
                })
        }
        // A headless row's items are its controls, so the first rung *is* the
        // control rung and there is nothing below it.
        [nth] if matches!(built.items, Items::Controls(_)) => {
            built.item().nth(nth, items).map(|control| Addressed::Of {
                item: None,
                nth,
                control,
            })
        }
        [nth] if nth <= items => Some(Addressed::Item(nth)),
        [nth, control] if nth <= items && matches!(built.items, Items::Alike(_)) => built
            .item()
            .nth(control, drawn(&[nth]))
            .map(|found| Addressed::Of {
                item: Some(nth),
                nth: control,
                control: found,
            }),
        [nth, through, control] if nth <= items && matches!(built.items, Items::Alike(_)) => {
            let above = built.item().nth(through, drawn(&[nth]))?;
            built
                .beneath(above)
                .nth(control, drawn(&[nth, through]))
                .map(|found| Addressed::Under {
                    item: nth,
                    through,
                    nth: control,
                    control: found,
                })
        }
        _ => None,
    }
}

/// Live deck state read directly at press time to avoid stale frame cache races.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Held {
    /// What the deck was last asked for, which is what the tally cycles from — see
    /// `view::Mixer::tally`, whose argument this is one surface along.
    pub requested: Tally,
    pub blend: BlendMode,
    pub mask: Mask,
    /// The angle the slot is already wearing, carried through unchanged (ADR-0203).
    pub mask_angle: f32,
}

/// Which way a press took a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Up,
    Down,
    /// `space` — back to the value the control was declared at, which is the one
    /// state a continuum has.
    Default,
}

/// Identifies host-managed continuous levels stepped by grammar arrow keys (ADR-0333).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Trim(u8),
    Fader(u8),
    /// The Master bay's out.
    Out,
    /// The Transport's free-run tempo, stepped a beat a minute (ADR-0350).
    Tempo,
    /// The Transport's exposure.
    Exposure,
    /// The Transport's latency offset.
    Offset,
}

/// Host operations requested by grammar key actions (ADR-0333).
#[derive(Debug, Clone, PartialEq)]
pub enum Asked {
    /// Nothing there answers this key, and the sentence that says why — a key that
    /// declines silently and a key that is not bound are the same experience
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    Nothing(&'static str),
    /// The address moved and nothing was asked of anything.
    Moved,
    /// One operation, named outright — the deck selection and the controls whose
    /// next state is this console's own affordance.
    Emitted(Operation),
    /// A move of the arrangement, which is not the vocabulary's: the fold `space`
    /// performs on a bay, and the Program head's solo.
    Panel(Op),
    /// The picture's on and off, which is one press asking for two things: the
    /// operation that names the output, and the fold that carries it out.
    Routed(Operation, Op),
    /// Stepped level adjustment dispatched to the host.
    Stepped { level: Level, step: Step },
    /// Steps Library head scope; host refreshes listing (ADR-0156).
    Scope,
    /// Loads selected Set onto active deck via host.
    Load,
    /// Dispatches audio input action to host (ADR-0156).
    Listened(AudioAsk),
    /// Dispatches arrangement action to host.
    Arranged(Ask),
}
