use super::*;

/// The digit that names a bay's head, which is the one digit that is not an
/// item.
///
/// A constant rather than a literal, because two very different things are
/// spelled `0` in this module — the head, and the first element of a path — and
/// only one of them is this.
pub const HEAD: usize = 0;

/// The Mixer bay, by the name the arrangement gives it — whose remembered item
/// is the deck selection.
///
/// A constant rather than a literal at the two methods that read it, for
/// [`crate::view::REGIONS`]' own reason: a bay nobody can find by name is a
/// pointer that silently stops pointing, and `tests/focus.rs` is what asserts
/// the ring holds it.
pub const MIXER: &str = "mixer";

/// The Library bay — whose remembered item is the row under the cursor and
/// whose head's remembered control is the marked scope. [`MIXER`]'s reason.
pub const LIBRARY: &str = "library";

/// Whether a region is one of the nine bays.
///
/// The four that are not are what a bay *contains*: the Program bay's picture
/// and preview row and the Inspector's two panes are its items, which is
/// ADR-0259's own finding — *"for two of the nine bays the arrangement's own
/// tree is the item list"*. So the walk stops at the bay and the four are
/// reached by a digit rather than by `Tab`.
///
/// A `match` with no wildcard, so a tenth kind is a compile error here rather
/// than a region that quietly never takes focus.
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

/// The bay of `layout` that contains `p`, or `None` if `p` is outside all visible bays
/// or on a divider grab margin.
///
/// Clicking inside a bay selects it as the active bay for keyboard shortcuts.
/// Divider grab margins are excluded so that divider resize gestures do not change
/// the focused bay.
pub fn bay_at(layout: &Layout, p: Point) -> Option<&'static Region> {
    if matches!(
        layout.hit(p, crate::panel::GRAB),
        karakuri_layout::Hit::Divider { .. }
    ) {
        return None;
    }
    for bay in ring(layout) {
        if let Some(id) = layout.find(bay.name) {
            if layout.visible(id) && layout.rect(id).contains(p) {
                return Some(bay);
            }
        }
    }
    None
}

/// Where the dashed focus ring goes on `bay`, or `None` where the arrangement
/// gives that bay no rectangle to put one on.
///
/// The head, or the whole row for a bay that draws none. The mock draws
/// `.wfocus` as a dashed sun outline and the deck selection as a solid lavender
/// ring, on purpose — *"drawing them the same way would erase which of the two
/// a reader is looking at"* — and `console.html`'s mark for a focused bay is
/// the head wearing it. The Transport and the Outputs row are headless
/// (ADR-0159), and ADR-0259 reads them the same way it reads their `0`: *"a
/// headless row, so `0` names the row itself"*, so the row stands in for the
/// head and the ring goes round the row.
///
/// `None` for a folded bay, and that is the drawing ADR-0259 leaves open. A
/// folded region has no rectangle and no divider is drawn beside it
/// ([ADR-0204](../../../docs/adr/0204-the-root-and-the-body-row-stay-unnamed-and-a-folded-root-is-not-hit-testable.md)),
/// so there is nothing on the panel to ring; `console.html` draws the mark that
/// is owed — the head alone — beside the note that defines it, and no bay is
/// folded in the panel it draws. The bay stays in the ring either way, which is
/// what `space` is for.
///
/// `layout` must be solved: `Layout::rect` refuses to answer from a dirty one.
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

/// Where the mark for a *folded* bay goes, or `None` for a bay that is not
/// folded, or one the arrangement is not drawing an edge for.
///
/// A folded bay keeps its place in the ring so that it can be opened, not so
/// that it can be operated — so it has to be markable, and a folded region has
/// no rectangle to mark. `docs/manual/console.html` draws what is owed: *the
/// head alone*, wearing the dashed ring, saying two things and no more — there
/// is a bay here, and `space` opens it.
///
/// The head is grown from the edge the fold leaves. A closed child keeps its
/// divider ([`Layout::is_placed`]), so it still solves to a rectangle — one
/// with no extent along its parent's axis, sitting exactly where the bay was.
/// This is that edge given a head's height, held inside the parent so that a
/// bay folded against the bottom of a column marks upward instead of off the
/// end of it. Every bay of this arrangement hangs in a column, so the width is
/// the edge's own; the parent's is the fallback for an arrangement whose bays
/// are a row, and it is written rather than assumed.
///
/// `None` unless the operator folded *this* bay. A bay inside a folded pane is
/// invisible and is not collapsed, and there is nothing of it on the panel to
/// mark — [`Layout::is_collapsed`] is the bit this asks and [`Layout::visible`]
/// is the one it does not.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one.
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

/// The bay a press that acts the same in every bay is addressed in, which is
/// not one of the nine.
///
/// `space` at bay level is the fold, and folding is a rule about *a bay* — so
/// the route it names is not the Mixer's or the Master's but every bay's, and
/// the page spells it `space &middot; in any bay`. It is a constant rather than
/// a literal for [`MIXER`]'s reason, and `crates/karakuri/src/main.rs`'s
/// `key_column` resolves the page's spelling to it.
///
/// It is a route and not a region, so nothing looks it up in
/// [`crate::view::REGIONS`]: a badge that named a bay here would be naming one
/// of nine places a press works.
pub const ANY: &str = "any";

/// A bay's remembered address: where the address is inside this bay, and the
/// nth thing it last named at each level.
///
/// The path is the record's own: `[]` is the bay itself, `[2]` is its second
/// item, `[2, 3]` is that item's third control, and [`HEAD`] in place of an
/// item names the bay's head. Digits count from one because they count what the
/// bay drew, which is ADR-0259's change from the four deck keys.
///
/// # Two fields, because they answer two questions
///
/// [`Address::at`] is *where the address is now* — the dashed ring, moved by a
/// digit and by `esc`. [`Address::remembered`] is *where it was* — the solid
/// ring, which is what makes the deck selection survive your hands being in the
/// library. One field could not be both: `esc` from deck B's fader leaves the
/// address at the Mixer and the selection on deck B, and a path that had been
/// popped would have taken the selection with it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Address {
    /// Where the address is now, from the bay down. Empty is the bay itself, which
    /// is where every bay starts and where `esc` stops.
    at: Vec<usize>,
    /// The nth item last named under each path, and never [`HEAD`] — see the module
    /// documentation, which is where that is argued.
    ///
    /// A map rather than one number, because a bay has more than one place to
    /// remember: the Library remembers which row it was on *and* which chip of its
    /// head is marked, and those are two levels of one address rather than two
    /// pointers.
    memory: BTreeMap<Vec<usize>, usize>,
}

impl Address {
    /// Where the address is now, from the bay down — empty for the bay itself.
    pub fn at(&self) -> &[usize] {
        &self.at
    }

    /// The nth item last named under `under`, or `None` for a path nothing has been
    /// named under.
    ///
    /// One-based, which is the digit that named it: `1` is the first thing the bay
    /// drew at that level. A caller that wants a position subtracts one, and the
    /// three places that arithmetic is written all subtract with a floor — never
    /// because zero is reachable, which it is not ([`Address::remember`] refuses
    /// [`HEAD`] and nothing else writes here), but because a wrapped `usize` in a
    /// paint path is a panic in an event handler and this crate's rule is the guard
    /// rather than the message.
    pub fn remembered(&self, under: &[usize]) -> Option<usize> {
        self.memory.get(under).copied()
    }

    /// Remember that the `nth` thing under `under` was named, and answer whether
    /// that moved anything.
    ///
    /// [`HEAD`] is refused rather than stored, which is the module's own rule: the
    /// head is not one of the things a bay lists, so there is nothing to remember
    /// about it and remembering it would overwrite the item this bay was on.
    ///
    /// The `bool` is `View::select`'s: a caller repaints on a move and not on a
    /// press.
    pub fn remember(&mut self, under: &[usize], nth: usize) -> bool {
        if nth == HEAD {
            return false;
        }
        self.memory.insert(under.to_vec(), nth) != Some(nth)
    }

    /// Descend to the `nth` thing below where the address is, remembering it on the
    /// way — which is what a digit does.
    ///
    /// Nothing here says the nth thing exists: the bay that draws it is what
    /// refuses a digit past its end, exactly as `View::select` refuses a deck the
    /// mixer has no strip for. This is the path and not the panel.
    pub fn down(&mut self, nth: usize) {
        self.remember(&self.at.clone(), nth);
        self.at.push(nth);
    }

    /// Ascends one level in the focus address hierarchy, returning true if not already at bay level.
    pub fn up(&mut self) -> bool {
        self.at.pop().is_some()
    }

    /// Back to the bay, without forgetting where it was.
    ///
    /// `esc` repeated is the ordinary way there and this is the other one: an
    /// address on something the bay has stopped drawing names nothing, and acting
    /// on whatever has taken that position is the failure
    /// [`crate::view::View::point_at`] refuses one level down. The memory is left
    /// alone, for [`Address::up`]'s reason — what a bay *was* on is the solid ring
    /// and is not what went stale.
    pub fn to_the_bay(&mut self) {
        self.at.clear();
    }

    /// Put the address on the `nth` item, remembering it — what an arrow does when
    /// the address had already descended to one.
    ///
    /// It is [`Address::down`] with the level replaced rather than pushed: a walk
    /// moves along a level and never into one.
    pub fn to_item(&mut self, nth: usize) {
        self.at.clear();
        self.down(nth);
    }

    /// Put the address on the `nth` thing at the level it has already reached,
    /// remembering it — [`Address::to_item`] at any depth, which is what an arrow
    /// does inside a card.
    ///
    /// `false` at the bay, where there is no level to walk along.
    pub fn to_row(&mut self, nth: usize) -> bool {
        if self.at.pop().is_none() {
            return false;
        }
        self.down(nth);
        true
    }
}

/// Which bay the keyboard is talking to, and what each bay remembers.
///
/// One pointer and nine addresses. The pointer is the dashed ring the mock
/// draws; an address is the solid one, seen once per bay.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Focus {
    /// The bay focus is on, or `None` for a console nobody has tabbed on yet —
    /// which resolves to *the first bay the traversal reaches* rather than to a bay
    /// named here.
    ///
    /// There is no unfocused state and this is not one. ADR-0259 rejected starting
    /// focus on a named bay because that is a second rule to keep in step with the
    /// walk — *"the first divider dragged would have parted them"* — so the start
    /// is the walk's own answer and [`Focus::bay`] is where it is asked. The same
    /// reading is what puts focus back in the ring when a bay leaves the
    /// arrangement.
    at: Option<&'static str>,
    /// Every bay that has ever been addressed, and what it remembers. A bay with no
    /// entry has been addressed by nobody, which is where a run starts.
    addresses: BTreeMap<&'static str, Address>,
}

impl Focus {
    /// The bay focus is on, resolved against `layout`.
    ///
    /// The first bay of the ring where nothing has been focused yet, and again
    /// where what was focused is no longer a bay of this arrangement — a pointer at
    /// a region that is not there is a ring drawn nowhere, which is
    /// `View::select`'s rule read at the bay instead of at a strip.
    ///
    /// `None` only for an arrangement with no bay in it at all, which no
    /// arrangement this crate builds is.
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

    /// Put focus on `bay`, and answer whether it moved.
    ///
    /// Resolved against `layout` for [`Focus::bay`]'s reason: a bay this
    /// arrangement does not hold is refused rather than stored, so nothing here can
    /// name a ring drawn nowhere.
    pub fn put(&mut self, layout: &Layout, bay: &str) -> bool {
        let Some(found) = ring(layout).into_iter().find(|found| found.name == bay) else {
            return false;
        };
        let was = self.bay(layout).map(|region| region.name);
        self.at = Some(found.name);
        was != Some(found.name)
    }

    /// `Tab`, and `shift-Tab` at `step` of `-1`: the next bay of the ring,
    /// wrapping.
    ///
    /// It always moves between bays, whatever depth the address had reached inside
    /// the one it leaves, and it never descends — which is why nothing here touches
    /// an [`Address`]. The bay it leaves keeps where it was, and that is the whole
    /// of what a remembered address is for.
    ///
    /// `shift-Tab` is this walk run backwards and nothing else. One key is the walk
    /// and the other is the walk reversed, so an operator who overshoots gets back
    /// exactly where they were — which is what a second rule for the backward
    /// direction would have cost.
    ///
    /// `false` where there is nothing to move to: an arrangement with one bay or
    /// none. A ring of one that wrapped onto itself would be a press that changed
    /// nothing and asked for a frame.
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

    /// `esc`: up one level of the focused bay's address.
    ///
    /// `false` at bay level, which is the refusal ADR-0259 asks to be said out
    /// loud: there is no rung below the bay and no unfocused state to fall out
    /// into, and a key that declines silently is indistinguishable from one that is
    /// not bound. It never leaves the ring and it never quits.
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

/// Where a bay's address has actually got to, resolved against what the bay is
/// drawing now.
///
/// A path is a list of digits and the bay is what says whether they name
/// anything: a listing that shrank between two frames leaves an address on a
/// row that is gone, exactly as it leaves the library cursor past the end.
/// `None` is that case, and the press says so rather than acting on the nearest
/// thing.
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
    /// A rung under one of the bay's own controls, in a bay whose items are its
    /// controls: the number of the control the address descended through, then the
    /// number of the row under it and which row it is. The Transport's two cards
    /// are the ones of these.
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

/// Where `at` has got to in `built`, against a bay whose rungs draw what
/// `drawn` says.
///
/// `drawn` is asked for a path and answers how many things the bay draws under
/// it: `[]` is how many items, `[n]` how many controls the nth item draws, and
/// `[n, c]` how many things are under that control. It is a closure rather than
/// a count because a bay is three deep and one number cannot say that.
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
        // **The rung under a head control**, which the Sequencer's chooser is
        // the one of: it is matched before the item arms below because `HEAD`
        // is zero and a path of three digits starting with one would otherwise
        // read as an item, a control and a thing under it.
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
        // **The rung under one of a headless row's own controls**, which the
        // Transport's two cards are: the control rung is the first, so a card
        // it puts down is the second. A card that is not down draws no rows,
        // so `drawn` answers zero and this resolves to nothing.
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

/// What the deck is holding right now, read at the press rather than off the
/// strip a frame copied.
///
/// The three states a strip's chips cycle, and the angle the mask is wearing.
/// This crate owns the cycle and not the reading: a scheduled fade landing
/// between the frame and the press would leave `view::Strip` a value the deck
/// has already left behind, which is `crates/karakuri/src/main.rs`'s own rule
/// at the three keys this replaces — *"read off the deck and not off the
/// strip"*.
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

/// Which level a press landed on, since the host names a different operation
/// for each and reads a different thing to step from.
///
/// The four here are the levels whose value is the world's — a deck's, the
/// engine's, the audio session's — so the host reads them and does the
/// arithmetic, which is ADR-0333's seam and `karakuri-cli`'s parity argument.
/// The two the console steps itself are the Inspector's, and they are not here
/// for `view::ParamGrip`'s reason: a parameter's address, range and value are
/// all on the pane the frame drew, and there is no second reading to take.
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

/// What a press of one of the four keys asks for.
///
/// The console resolves the address and says what was landed on; the window
/// loop names the operation, because the levels above need the world in front
/// of them and two of the answers reach the store (ADR-0333).
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
    /// A level, and which way the press went. The host reads the value off the
    /// world and names the destination, because the size of a step and the clamp on
    /// it are its arithmetic and `karakuri-cli`'s.
    Stepped { level: Level, step: Step },
    /// The Library head's scope, stepped — the host performs it and re-reads the
    /// listing, which is a directory read and not a thing this crate can do at all
    /// (ADR-0156).
    Scope,
    /// The Set under the Library's cursor, loaded onto the selected deck — the
    /// host's for the scope's reason, and because a preset row is taken into the
    /// store on the way.
    Load,
    /// A press on the audio-in pill or on one of its rows, in that control's own
    /// words. The host performs it through the method a pointer press already
    /// reaches, because opening the card enumerates the machine's inputs and this
    /// crate takes no device (ADR-0156).
    Listened(AudioAsk),
    /// A press on the arrangement pill or on one of its menu rows, in that
    /// control's own words. The host performs it through the method a pointer press
    /// already reaches: the names filed are a directory and the save writes a file.
    Arranged(Ask),
}
