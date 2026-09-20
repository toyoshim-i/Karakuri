use super::*;

impl View {
    /// Which pane head is taking letters, and what is in it — or `None` for a
    /// console where nothing is being named, which is every test in this crate that
    /// does not say otherwise.
    pub fn naming_set(&self) -> Option<&Naming> {
        self.naming.as_ref()
    }

    /// What pane `index`'s head is taking letters into, or `None` where it is
    /// reading. This is what [`deck_name`] and [`inspector_into`] each ask, so that
    /// one head is asking and the other is not.
    pub fn naming_set_in(&self, index: usize) -> Option<&str> {
        self.naming
            .as_ref()
            .filter(|naming| naming.pane == index)
            .map(Naming::typed)
    }

    /// What is being typed into the head of whichever pane is showing `deck`, or
    /// `None` where no head is asking for a name over that deck.
    ///
    /// What it is for is a node's keep (ADR-0338, decision 4): the capsule on a
    /// node group's head types nothing and takes a stamp, and a head that *is*
    /// taking letters is what a keep from that pane files under — which is
    /// ADR-0128's two routes drawn on one capsule, exactly as the deck's own `keep`
    /// draws them.
    ///
    /// The pane is found by the deck rather than carried, which is
    /// [`View::named_set`]'s own rule: the gesture spans frames, so what is filed
    /// is what the head says it is filing *now*. An empty buffer answers
    /// `Some("")`, and that is the field's rule and not this method's — the console
    /// emits what was typed, including nothing, and the wall is where the file is
    /// written (P-0090).
    pub fn naming_over(&self, deck: u8) -> Option<String> {
        let naming = self.naming.as_ref()?;
        let pane = self.inspector.get(naming.pane)?;
        (pane.deck == usize::from(deck)).then(|| naming.typed().to_owned())
    }

    /// Ask for a name in pane `index`'s head, starting from empty.
    ///
    /// Starting from empty rather than from the material's name. The run under the
    /// caret read `drift_night` a moment ago and the field does not keep it: a
    /// buffer seeded with what was there is a name an operator commits by pressing
    /// return once, which is the shape of an overwrite nobody typed. What ADR-0128
    /// makes an instruction is a name that was *typed*.
    pub fn name_set(&mut self, index: usize) {
        self.naming = Some(Naming {
            pane: index,
            typed: String::new(),
        });
    }

    /// Take the field away, typed name and all, which is what escape asks and what
    /// a press somewhere else asks. [`Arrangement::shut`]'s sentence: a name
    /// abandoned half-typed is not kept for the next time, because the buffer is
    /// the gesture and the gesture ended.
    pub fn stop_naming_set(&mut self) {
        self.naming = None;
    }

    /// One character into the name being typed, and `false` where no head was
    /// asking for one. [`Arrangement::typed`]'s rule and its refusal: control
    /// characters are not a name and never reach the buffer, because a newline is
    /// Return arriving as text and that is the commit.
    pub fn type_into_name(&mut self, c: char) -> bool {
        match (&mut self.naming, c.is_control()) {
            (Some(naming), false) => {
                naming.typed.push(c);
                true
            }
            _ => false,
        }
    }

    /// The last character back out again, and `false` where there was nothing to
    /// take — no head asking, or an empty name.
    pub fn rub_out_of_name(&mut self) -> bool {
        match &mut self.naming {
            Some(naming) => naming.typed.pop().is_some(),
            None => false,
        }
    }

    /// The name is finished, and this is what it asks for: the deck that head is
    /// showing, filed under what was typed.
    ///
    /// # The deck is read at the commit and not at the press
    ///
    /// [`KeepPill`] carries the deck it was measured for because its press is one
    /// instant; this gesture spans frames, and what is filed has to be the deck the
    /// head says it is filing *now*. So the pane is looked up again and `None` is a
    /// pane that has gone — a console handed a shorter [`View::inspector`] while
    /// somebody was typing — where the field is taken away and nothing is emitted,
    /// rather than a keep landing on a deck whose head is no longer on screen.
    ///
    /// # It refuses nothing else
    ///
    /// An empty name arrives here as an empty name and leaves as one, which is
    /// [`Arrangement`]'s rule at the same seam: `id` is one path component and the
    /// wall is where the bytes are written. A name typed twice overwrites, which is
    /// ADR-0128 and is not this control's to soften.
    ///
    /// The field is taken away whether or not the name is any good, for
    /// `Readout::named`'s reason one bay along: a refusal is said out loud by
    /// whoever refuses it, and a field left standing over the refusal would be the
    /// panel asking the question again without saying the answer.
    pub fn named_set(&mut self) -> Option<Operation> {
        let naming = self.naming.take()?;
        let deck = self.inspector.get(naming.pane)?.deck as u8;
        Some(Operation::SaveSet {
            deck,
            id: Some(naming.typed),
        })
    }

    /// Which `uses` line's card is down, or `None` — see [`View::wiring_open`] the
    /// field.
    pub fn wiring_open(&self) -> Option<(usize, usize, usize)> {
        self.wiring_open
    }

    /// Put one `uses` line's card down, and answer whether anything moved.
    ///
    /// Refused for a console with no pane, which is [`View::open_target`]'s own
    /// guard and its reason: a card with no line under it offers nothing to pick
    /// and nothing to leave by, and `input::claim`'s rule 2 would hand it every
    /// press until a second one shut it.
    pub fn open_wiring(&mut self, pane: usize, node: usize, input: usize) -> bool {
        if self.inspector.get(pane).is_none() {
            return false;
        }
        let at = Some((pane, node, input));
        let moved = self.wiring_open != at;
        self.wiring_open = at;
        moved
    }

    /// Put it away, and answer whether one was down.
    pub fn shut_wiring(&mut self) -> bool {
        let was = self.wiring_open.is_some();
        self.wiring_open = None;
        was
    }

    /// How far one Inspector pane is scrolled, as it is stored — the number
    /// [`inspector`] clamps and never the one it clamped.
    ///
    /// Zero for a pane index past [`PANES`], which is a caller's error and not a
    /// state: the bay has two panes and `PANE_NAMES` is what says so.
    pub fn scroll_in(&self, pane: usize) -> f32 {
        self.scroll.get(pane).copied().unwrap_or(0.0)
    }

    /// Turn one pane's wheel by `by` pixels, positive down the list, and answer
    /// whether the stored position moved.
    ///
    /// # Two clamps, and only one of them is here
    ///
    /// This one is against the content — how tall everything the deck publishes
    /// comes to — and it is a reading of the deck rather than of a viewport, so a
    /// stored position bounded by it is not a position any resize can rewrite.
    /// Without it a wheel spun over a short Set would put the number in the
    /// thousands and an operator would have to spin it all the way back before
    /// anything moved, which is *"a control you cannot see being moved"* by another
    /// name.
    ///
    /// The other clamp is against the pane's own height and belongs where the pane
    /// is laid out — [`InspectorPane::scroll`], which is
    /// [P-0082](../../../../docs/principles/0082-looking-never-writes-back.md): a
    /// shorter pane draws less of the same position and stores nothing, so dragging
    /// it back reproduces the picture exactly rather than nearly (ADR-0250's
    /// argument one region in).
    ///
    /// A pane this console is not showing anything in refuses the wheel rather than
    /// storing a position for it, which is [`View::point_at`]'s rule: what a
    /// pointer can be at is something drawn.
    pub fn scroll_by(&mut self, pane: usize, by: f32) -> bool {
        let Some(showing) = self.inspector.get(pane) else {
            return false;
        };
        let Some(at) = self.scroll.get_mut(pane) else {
            return false;
        };
        let content = content_h(&showing.nodes);
        let next = (*at + by).clamp(0.0, content);
        let moved = next != *at;
        *at = next;
        moved
    }

    /// Which deck one Inspector pane is pointed at — see [`View::pane_deck`] the
    /// field, which is where the argument is.
    ///
    /// This is what the host reads to fill the pane, which is
    /// [`View::target_deck`]'s arrangement one bay along: the pointer is the
    /// console's and what is under it is the host's answer to *what is that deck
    /// playing*.
    ///
    /// Deck A for a pane index past [`PANES`], which is a caller's error and not a
    /// state — [`View::scroll_in`]'s own rule.
    ///
    /// [`View::pane_deck`]: Self::pane_deck
    pub fn pane_deck(&self, pane: usize) -> u8 {
        self.pane_deck.get(pane).copied().unwrap_or(0)
    }

    /// Every pane's target at once, which is what a host reads before it fills
    /// [`View::inspector`]: the panes are written through a `&mut` of that field,
    /// so asking pane by pane while it is borrowed is a second borrow of this
    /// struct. It is two bytes and `Copy`.
    pub fn pane_decks(&self) -> [u8; PANES] {
        self.pane_deck
    }

    /// Point one pane at `deck`, put the card away, and answer whether anything
    /// moved.
    ///
    /// A deck the mixer has no strip for is refused, which is [`View::select`]'s
    /// rule and [`View::aim_at`]'s read a third time rather than a third rule: the
    /// head says which deck it is showing, so a target past the deck's slots would
    /// be a letter naming a deck with nothing under it.
    ///
    /// It refuses rather than clamping, for `select`'s reason: a pick of deck D at
    /// a two-slot deck means *deck D*, and clamping would point the pane at deck B
    /// — a different deck than the one asked for.
    ///
    /// Nothing else moves, and that is the whole of what this mark is for: not the
    /// deck selection, not the pane next door, not the Library bay's load target.
    /// Each of those is a pointer of its own with a writer of its own, and this one
    /// touches none of them.
    ///
    /// The card goes away here, which is [`View::aim_at`]'s clause: a pick is one
    /// gesture and this is the whole of it, so a card left down would go on
    /// claiming every press on the console. It is put away even where the pane did
    /// not move — picking the deck a pane already shows is still a hand finishing
    /// what it started.
    ///
    /// The `bool` is [`View::select`]'s: a caller repaints on a move and not on a
    /// press.
    pub fn point_pane(&mut self, pane: usize, deck: u8) -> bool {
        if usize::from(deck) >= self.mixer.len() {
            return false;
        }
        let Some(at) = self.pane_deck.get_mut(pane) else {
            return false;
        };
        let moved = *at != deck || self.pane_open.is_some();
        *at = deck;
        self.pane_open = None;
        moved
    }

    /// Which pane head's pulldown is down, or `None` — see [`View::pane_open`] the
    /// field.
    ///
    /// [`View::pane_open`]: Self::pane_open
    pub fn pane_target_open(&self) -> Option<usize> {
        self.pane_open
    }

    /// Put one pane head's card down, and answer whether anything moved.
    ///
    /// Refused for a pane this console is not showing, which is
    /// [`View::open_wiring`]'s own guard and its reason: a card with no head under
    /// it offers nothing to pick and nothing to leave by, and `input::claim`'s rule
    /// 2 would hand it every press until a second one shut it.
    pub fn open_pane_target(&mut self, pane: usize) -> bool {
        if pane >= PANES || self.inspector.get(pane).is_none() {
            return false;
        }
        let moved = self.pane_open != Some(pane);
        self.pane_open = Some(pane);
        moved
    }

    /// Put it away, and answer whether one was down.
    pub fn shut_pane_target(&mut self) -> bool {
        let was = self.pane_open.is_some();
        self.pane_open = None;
        was
    }

    /// What one pane head's pulldown is, laid out — the mark and, while it is down,
    /// the card under it.
    ///
    /// Read once for the frame and handed to the paint and to the press, exactly as
    /// [`View::target`] is: the mark that is drawn and the mark a press lands on
    /// are one derivation of one reading.
    ///
    /// `None` is a head with no run in it, which is [`deck_name`]'s refusal — see
    /// [`pane_target`].
    pub fn pane_pulldown(
        &self,
        ctx: &egui::Context,
        at: &InspectorPane,
        pane: &Pane,
        index: usize,
    ) -> Option<PaneTarget> {
        let policy = self
            .slot_policies
            .get(pane.deck)
            .copied()
            .unwrap_or_default();
        let mcp = slot_mcp_pill(ctx, at, pane, policy).map(|p| p.pill);
        pane_target(
            ctx,
            PaneTargetCtx {
                at,
                pane,
                index,
                naming: self.naming_set_in(index),
                decks: self.mixer.len(),
                open: self.pane_open == Some(index),
                mcp,
            },
        )
    }
}
