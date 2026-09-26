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

    /// Returns the text being typed into the head of the pane displaying `deck`, or `None`.
    pub fn naming_over(&self, deck: u8) -> Option<String> {
        let naming = self.naming.as_ref()?;
        let pane = self.inspector.get(naming.pane)?;
        (pane.deck == usize::from(deck)).then(|| naming.typed().to_owned())
    }

    /// Ask for a name in pane `index`'s head, starting from empty per ADR-0128.
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

    /// Commits the typed name and returns a [`Operation::SaveSet`] operation.
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

    /// Lowers one `uses` line's card, returning whether state changed.
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

    /// Stored scroll offset for pane `index`, clamped to content height.
    pub fn scroll_in(&self, pane: usize) -> f32 {
        self.scroll.get(pane).copied().unwrap_or(0.0)
    }

    /// Scrolls pane `index` by `by` pixels, clamped to content height per P-0082 and ADR-0250.
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

    /// Returns which deck the specified Inspector pane is pointed at.
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

    /// Targets pane `index` to `deck`, dismisses the card, and returns whether state changed.
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

    /// Opens the deck selection card for pane `index` if valid, returning whether state changed.
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

    /// Laid-out target pulldown mark and active card for pane `index`, or `None`.
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
