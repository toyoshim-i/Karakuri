use super::*;

// ---------------------------------------------------------------------------
// The View struct
// ---------------------------------------------------------------------------

pub struct View {
    pub room: Room,
    /// Whether the external projector window is open (ADR-0156).
    pub projector: bool,
    /// Whether plugin sink 0 (Syphon) is active.
    pub plugin: bool,
    /// Whether plugin sink 0 is available on this machine.
    pub plugin_available: bool,
    /// Discovered display name of plugin sink 0, if available (e.g. "Spout", "Syphon").
    pub plugin_name: Option<&'static str>,
    /// Texture to render in the Program bay for this frame, or `None` if engine-less.
    /// Updated per frame by the device owner (ADR-0156).
    pub picture: Option<Picture>,
    /// Textures for the four deck preview cells in slot order, or `None` if empty/engine-less.
    /// Renders slot material regardless of residency (ADR-0240, ADR-0258).
    pub previews: [Option<Picture>; DECKS],
    /// Indicates whether each deck slot exceeded frame budget and stopped updating (ADR-0269, ADR-0316).
    /// Prevents displaying unmarked still frames without user warning.
    pub overloaded: [bool; DECKS],
    /// Governor budgeted frame cost per deck slot from the last governor report (ADR-0156).
    /// Used by [`caption_into`] to paint the risk dot band.
    pub costs: [Option<Budgeted>; DECKS],
    /// Transport row readings (tempo, beat, frame cost) for this frame, or `None` (ADR-0156).
    pub transport: Option<Transport>,
    /// Active arrangement name, stored presets, and dropdown menu state for the transport row.
    pub arrangement: Arrangement,
    /// Audio input device selection and card state, or `None` if unconfigured (ADR-0156).
    pub audio: Option<AudioIn>,
    /// Indicates whether MIDI learn mode is armed, illuminating the transport learn pill (ADR-0336, Rule 4).
    pub learn: bool,
    /// Active MIDI map name displayed in the transport row, or `None` (ADR-0156).
    pub map: Option<MapPill>,
    /// Beat tracker group state (latency offset and grid octave shifts), or `None` (ADR-0156).
    pub tracker: Option<Tracker>,
    /// Active look parameters (`karakuri_engine::frame::Look`) for this frame, or `None` (ADR-0156).
    pub look: Option<Look>,
    /// Master bus output level reading at entry, or `None` (ADR-0156, ADR-0224, ADR-0317).
    pub master_out: Option<f32>,
    /// Snapshot of the master chain slots and parameters, or `None` (ADR-0156).
    pub master_chain: Option<Chain>,
    /// Whether the master chain is currently building on the background worker thread.
    /// When true, the Master bay draws an in-flight building indicator badge.
    pub master_chain_building: bool,
    /// List of available `kind L5` library procedures offered by `+ add` (ADR-0156).
    pub chain_add: Vec<AddChoice>,
    /// State of each mixer strip in slot order, or empty if no deck is attached (ADR-0156).
    pub mixer: Vec<Strip>,
    /// Whether the mixer bay needs a redraw due to an operation changing mixer state.
    pub mixer_dirty: bool,
    /// Listed Set names for the current scope, populated via directory reads (ADR-0156, P-0091).
    pub library: Vec<String>,
    /// Kind and layer badges corresponding to each listed library row (ADR-0156, ADR-0338, P-0091).
    pub kinds: Vec<RowKind>,
    /// Set IDs marked as favourites/starred in `<store>/favourites.json` (ADR-0156, ADR-0299).
    pub starred: std::collections::BTreeSet<String>,
    /// ID of the Set currently aimed at the load pulldown's target deck (ADR-0156, ADR-0305).
    pub aimed: Option<String>,
    /// Available library scopes displayed as scope chips (ADR-0156).
    pub scopes: Vec<Scope>,
    /// Candidate node names available to narrow the library listing (ADR-0156, P-0091).
    pub holds: Vec<String>,
    /// Disk path currently pointed to by the library for external assets (ADR-0156, ADR-0311, P-0091).
    pub folder: Option<String>,
    /// Hovered folder path during an active drag-and-drop operation (ADR-0275, P-0091).
    pub incoming: Option<String>,
    /// Candidates in the staging lane awaiting swap or verdict (ADR-0156).
    pub staging: Vec<Candidate>,
    /// Inspector pane state per deck slot, populated on Set publish (ADR-0156, ADR-0200, ADR-0216).
    pub inspector: Vec<Pane>,
    /// Program bay rendering dimensions `(width, height)` used for layout arrangement (ADR-0156).
    pub canvas: (u32, u32),
    /// Opened capability classes for MCP model access (ADR-0156, ADR-0235).
    pub opening: Open,
    /// Slot-level MCP modification policy for each deck slot.
    pub slot_policies: [SlotPolicy; DECKS],
    /// Animation time offset elapsed on the host clock (P-0092).
    pub phase: Phase,
    /// Active keyboard focus and remembered selection per bay (ADR-0259, ADR-0305, ADR-0332).
    pub(crate) focus: Focus,
    /// Deck targeted by Library bay load controls (ADR-0305).
    pub(crate) target: u8,
    /// Whether the Library bay target deck pulldown menu is currently open (Rule 2).
    pub(crate) target_open: bool,
    /// Active inspector wiring card coordinates `(pane, node, input)`, if open (ADR-0305, Rule 2).
    pub(crate) wiring_open: Option<(usize, usize, usize)>,
    /// Whether the staging `+ lane` chooser card is currently open (Rule 2).
    pub(crate) lane_open: bool,
    /// Whether the Master bay's `+ add` chooser is down — [`View::lane_open`]'s
    /// field one bay along. [`View::open_chain_add`] and
    /// [`View::shut_chain_add`] are the only ways in.
    pub(crate) chain_add_open: bool,
    /// Library row whose contextual action menu is open, or `None`.
    pub(crate) menu_row: Option<usize>,
    /// Vertical scroll offset of the Library bay in logical pixels (ADR-0307, ADR-0312).
    pub(crate) library_scroll: f32,
    /// Active Set reading under inspection in Library bay (ADR-0156, P-0091).
    pub(crate) reading: Option<Reading>,
    /// Selected index in [`View::holds`] used to filter the library listing.
    pub(crate) holds_at: Option<usize>,
    /// Active kind filter flags selected in the Library bay (ADR-0338).
    pub(crate) showing: LibraryKinds,
    /// Active text naming session for a pane head, or `None`.
    pub(crate) naming: Option<Naming>,
    /// Vertical scroll offsets per Inspector pane in logical pixels (ADR-0307, P-0082).
    pub(crate) scroll: [f32; PANES],
    /// Target deck slot displayed in each Inspector pane (ADR-0338).
    pub(crate) pane_deck: [u8; PANES],
    /// Inspector pane whose deck selection dropdown is open, or `None` (Rule 2).
    pub(crate) pane_open: Option<usize>,
    /// Selected transition mode, shape, and duration parameters for upcoming wipes/fades.
    pub(crate) transition: TransitionSettings,
    /// Sequencer bay pattern and trigger state snapshot for this frame, or `None` (ADR-0156).
    pub sequencer: Option<Sequenced>,
    /// Prompt bay terminal and agent CLI session state.
    pub prompt: PromptState,
    pub(crate) placed: Vec<Placed>,
}
