use crate::Operation;

/// A class of operations the operator can open, and the bay whose head opens
/// it. ADR-0235 names four and each has a bay.
///
/// The classes are drawn off P-0094's question — *what does this do at its
/// worst, on the frame it goes wrong, while the operator's attention is on the
/// room?* — and not off the nouns. That is why [`Operation::WriteProcedure`] is
/// open although it rewrites what a live deck is drawing: it is priced before
/// it is built, lands at a frame boundary and rolls back on its own, so it
/// fails to be an *unpriced, immediate, irreversible* write on every count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// What a deck that is live is drawing. Opened at the head of the Program bay,
    /// which is where what is on air lives.
    LiveDeck,
    /// The mix faders. Opened at the head of the Mixer bay.
    MixFaders,
    /// The master effects. Opened at the head of the Master bay.
    MasterEffects,
    /// Inputs and outputs, routed, enabled and disabled. Opened at the head of the
    /// Outputs row.
    InputsAndOutputs,
}

impl Class {
    /// Every class, for a caller drawing one indicator per class.
    pub const ALL: &'static [Class] = &[
        Class::LiveDeck,
        Class::MixFaders,
        Class::MasterEffects,
        Class::InputsAndOutputs,
    ];

    /// The class in the words ADR-0235 named it in, which is the words the refusal
    /// says it in.
    pub fn title(self) -> &'static str {
        match self {
            Class::LiveDeck => "what a deck that is live is drawing",
            Class::MixFaders => "the mix faders",
            Class::MasterEffects => "the master effects",
            Class::InputsAndOutputs => "inputs and outputs, routed, enabled and disabled",
        }
    }

    /// Where the operator opens it, which the refusal has to say or the model
    /// reports the instrument as incapable rather than as closed
    /// ([P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md)).
    pub fn bay(self) -> &'static str {
        match self {
            Class::LiveDeck => "Program",
            Class::MixFaders => "Mixer",
            Class::MasterEffects => "Master",
            Class::InputsAndOutputs => "Outputs",
        }
    }

    /// Where the operator finds the pill, in the words the refusal says it in — and
    /// three of the four are *the head of the … bay* while one is not.
    ///
    /// The Outputs row has no head to put an indicator in. `karakuri-console` says
    /// so outright of `Kind::Outputs`: it is a label *"inside a row that has no
    /// head at all (ADR-0159), so it is that typography and none of that structure:
    /// no hairline under it, no pills or grip beside it."* So the pill sits beside
    /// the word that stands in for a head, and `docs/manual/console.html` specifies
    /// it there and says why.
    ///
    /// A refusal naming a place that does not exist is worse than one naming none,
    /// because a model repeats it to the person sitting there and sends them
    /// looking for a head. That is the whole reason this is a second function
    /// rather than a format string over [`Class::bay`].
    pub fn opened_at(self) -> &'static str {
        match self {
            Class::LiveDeck => "the head of the Program bay",
            Class::MixFaders => "the head of the Mixer bay",
            Class::MasterEffects => "the head of the Master bay",
            Class::InputsAndOutputs => "the Outputs row, which has no head",
        }
    }
}

/// Operations that are closed by safety policy without belonging to any unlockable bay class (ADR-0235).
///
/// These operations are unconditionally refused by the safety gate and cannot be
/// unlocked via [`Open`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unclassed {
    /// The clock. *"Unpriced, immediate, irreversible — a tap sets the phase and
    /// there is no un-tapping it."*
    Clock,
    /// [`Operation::Quit`]. *"It does not risk stopping the performance, it stops
    /// it."*
    Quitting,
    /// [`Operation::SelectDeck`]. Closed by P-0094's second half: it decides which
    /// deck the operator's *next key press* lands on.
    Selection,
    /// The sequencer's lanes. A lane emits operations, so one pointed at a fader
    /// and unmuted is a mix write on a delay.
    Lanes,
    /// [`Operation::SetAuthority`]. A permission an actor can grant itself is not a
    /// permission, and ADR-0235 recommends this one is never openable while leaving
    /// that the maintainer's.
    Authority,
}

impl Unclassed {
    /// The group in the words the refusal says it in.
    pub fn title(self) -> &'static str {
        match self {
            Unclassed::Clock => "the clock",
            Unclassed::Quitting => "quitting",
            Unclassed::Selection => "which deck a key press lands on",
            Unclassed::Lanes => "the sequencer's lanes",
            Unclassed::Authority => "who may move a node",
        }
    }
}

/// Which reading a row's class turns on, so a refusal can say what nobody
/// handed over rather than that something was missing.
///
/// `karakuri_operation_record::Reading`'s shape and its reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// [`Running::live`], which [`Operation::LoadSet`]'s class turns on.
    Live,
}

/// State read about the instrument for gate operations whose classification
/// depends on both the operation and its target state (ADR-0235).

#[derive(Debug, Clone, Copy, Default)]
pub struct Running<'a> {
    /// The decks whose slot is live, as somebody read them from the engine.
    ///
    /// `None` is *nobody read it*, and a row that turns on it is then refused
    /// rather than guessed — the reading is missing, the safe answer is the closed
    /// one, and [`refusal`] says which reading was missing rather than saying
    /// *closed* and leaving the caller nothing to act on.
    pub live: Option<&'a [u8]>,
}

impl<'a> Running<'a> {
    /// Nothing read. See [`Running::live`].
    pub fn unread() -> Running<'a> {
        Running { live: None }
    }

    /// The decks that are live, read.
    pub fn live(decks: &'a [u8]) -> Running<'a> {
        Running { live: Some(decks) }
    }
}

/// Where one operation stands with the audit, before the operator's opening is
/// consulted.
///
/// [`standing`] answers this; [`audit`] is what turns it and an [`Open`] into a
/// yes or a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Open by default — twenty-three rows. Nothing gates it and nothing ever did.
    Open,
    /// Closed by default, and the operator opens this class at the head of
    /// [`Class::bay`].
    Closed(Class),
    /// Closed by default, and no bay opens it. See [`Unclassed`].
    ClosedUnclassed(Unclassed),
    /// Closed, because the reading its class turns on was not taken. See
    /// [`Running`].
    Unread(Reading),
}

/// Which classes the operator has opened.
///
/// The fields are private and every one is `false`, so closed by default is the
/// type's own `Default`: there is no way to write down an `Open` that starts
/// open, and the only route to one is [`Open::with`], which has to name the
/// class.
///
/// [`Unclassed`] groups have no field here on purpose — see that type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Open {
    live_deck: bool,
    mix_faders: bool,
    master_effects: bool,
    inputs_and_outputs: bool,
}

impl Open {
    /// All four closed, which is what a run starts with.
    pub const CLOSED: Open = Open {
        live_deck: false,
        mix_faders: false,
        master_effects: false,
        inputs_and_outputs: false,
    };

    /// This opening with one class set. Names a state and never a direction, which
    /// is
    /// [P-0090](../../../docs/principles/0090-a-surface-offers-it-never-decides.md)
    /// applied to a setting the vocabulary does not own: a bay-head pill can be a
    /// toggle, and what it writes still says which state it means.
    pub fn with(self, class: Class, open: bool) -> Open {
        let mut next = self;
        match class {
            Class::LiveDeck => next.live_deck = open,
            Class::MixFaders => next.mix_faders = open,
            Class::MasterEffects => next.master_effects = open,
            Class::InputsAndOutputs => next.inputs_and_outputs = open,
        }
        next
    }

    /// Whether the operator has opened this class.
    pub fn holds(self, class: Class) -> bool {
        match class {
            Class::LiveDeck => self.live_deck,
            Class::MixFaders => self.mix_faders,
            Class::MasterEffects => self.master_effects,
            Class::InputsAndOutputs => self.inputs_and_outputs,
        }
    }
}

/// An operation that has been through the audit, and the only thing a performer
/// will take.
///
/// The field is private and [`audit`] is the only function that builds one, so
/// a caller in another crate has no way to perform an operation that was not
/// checked. That is the structural half of *"an audit skipped on one path is
/// the whole mechanism gone"*.
#[derive(Debug, Clone, Copy)]
pub struct Allowed<'a>(pub(crate) &'a Operation);

impl<'a> Allowed<'a> {
    /// The operation this gate check verified.
    pub fn operation(&self) -> &'a Operation {
        self.0
    }
}
