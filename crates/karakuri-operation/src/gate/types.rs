use crate::Operation;

/// A protected operation class unlockable by an operator at a specific console bay (ADR-0235).
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

    /// Display location where the operator toggles this class pill in the UI.
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
    /// Active live deck indices read from the engine, or `None` if unread.
    pub live: Option<&'a [u8]>,
}

impl<'a> Running<'a> {
    /// Constructs a `Running` state indicating live decks were unread.
    pub fn unread() -> Running<'a> {
        Running { live: None }
    }

    /// Constructs a `Running` state with known live deck indices.
    pub fn live(decks: &'a [u8]) -> Running<'a> {
        Running { live: Some(decks) }
    }
}

/// The policy standing of an operation before consulting operator authorizations (ADR-0235).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Open by default without gating.
    Open,
    /// Closed by default unless the specified class is explicitly opened.
    Closed(Class),
    /// Closed by default and cannot be unlocked (see [`Unclassed`]).
    ClosedUnclassed(Unclassed),
    /// Refused because a prerequisite runtime state reading was missing.
    Unread(Reading),
}

/// Operator authorizations opening protected operation classes (ADR-0235).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Open {
    live_deck: bool,
    mix_faders: bool,
    master_effects: bool,
    inputs_and_outputs: bool,
}

impl Open {
    /// All classes closed (default state).
    pub const CLOSED: Open = Open {
        live_deck: false,
        mix_faders: false,
        master_effects: false,
        inputs_and_outputs: false,
    };

    /// Returns a new `Open` configuration with the specified class set to `open`.
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

    /// Returns `true` if the operator has opened the specified class.
    pub fn holds(self, class: Class) -> bool {
        match class {
            Class::LiveDeck => self.live_deck,
            Class::MixFaders => self.mix_faders,
            Class::MasterEffects => self.master_effects,
            Class::InputsAndOutputs => self.inputs_and_outputs,
        }
    }
}

/// A validated operation that has passed the gate audit.
///
/// Can only be constructed via [`super::rules::audit`].
#[derive(Debug, Clone, Copy)]
pub struct Allowed<'a>(pub(crate) &'a Operation);

impl<'a> Allowed<'a> {
    /// The operation this gate check verified.
    pub fn operation(&self) -> &'a Operation {
        self.0
    }
}
