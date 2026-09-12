use super::*;

// ---------------------------------------------------------------------------
// The Outputs row
// ---------------------------------------------------------------------------

/// The word at the head of the Outputs row, in the source's own capitalisation
/// for the reason [`Kind::Bay`]'s title is: the mock upper-cases in CSS, and
/// that is done at paint time here so the word a reader searches for is the
/// word in the source.
///
/// It is not a [`bay_head`]. The mock's markup is an inline span with
/// `.bay-head`'s type on it — `font-size: 10px`, `letter-spacing: 0.16em`,
/// `text-transform: uppercase`, `--c-faint` — inside a row that has no head at
/// all (ADR-0159), so it is that typography and none of that structure: no
/// hairline under it, no pills or grip beside it, and the row's own padding
/// rather than a head's.
const OUTPUTS_LABEL: &str = "Outputs";

/// The one sink the console has, and the manual's name for it: *"The picture in
/// the Program bay is a sink like any other and is the first row."*
const PROGRAM_VIEW: &str = "program view";

/// The projector window's chip, and the whole of its name.
///
/// The mock draws `projector · DELL U2720Q`, and the display half is not built:
/// naming which screen a window is on wants a list of displays this program
/// does not read, and a label that carries a monitor's model is a label that
/// changes when the cable does — which is exactly why
/// [`karakuri_operation::Output`] is a closed list and not a string. The chip
/// says what it is; the manual's tooltip says which display it would name.
const PROJECTOR: &str = "projector";

/// The two plugin sinks the mock draws, and what they say with no plugin
/// loaded.
///
/// `docs/plugins.md` specifies the process and the handshake and nothing
/// implements them, so there is no manifest to read a sink out of and neither
/// of these is switchable. They are drawn rather than omitted for the mock's
/// own reason, which its `NDI · no plugin` chip already carried: *a control
/// explaining that the thing it would switch is not installed* is a state, and
/// a row that simply did not mention Syphon would leave an operator wondering
/// whether this program has heard of it.
const PLUGIN_SINKS: [&str; 2] = ["Syphon · no plugin", "NDI · no plugin"];

/// One chip in the Outputs row that is not the program view.
///
/// The program view is not one of these and the asymmetry is the model rather
/// than a shortcut: its on/off is [`Layout::visible`] on the picture's node —
/// read, never stored ([`Outputs::on`]) — and no other output has a layout node
/// at all. A uniform array would have needed an `Option<NodeId>` on every chip,
/// which would say a projector window might have one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SinkChip {
    /// What this chip names, and what a press asks for by name — see
    /// [`karakuri_operation::Output`], which is where the closed list is argued.
    pub output: Output,
    /// The capsule, dot and name together: what a press has to land in, the same
    /// whole-chip target the program view's is.
    pub chip: Rect,
    /// The `.dot` inside it.
    pub dot: Rect,
    /// Whether this output is on. `false` until somebody says otherwise — this
    /// crate has no window and no plugin host, so [`Outputs::told`] is how the
    /// answer arrives.
    pub on: bool,
    /// Whether there is anything behind it at all. `false` draws `.absent` — dim,
    /// and not a control — which is the mock's own state for a sink whose plugin is
    /// not loaded.
    pub present: bool,
    /// What is written in it.
    pub name: &'static str,
}

impl SinkChip {
    /// What a press on this chip asks for, or `None` where there is nothing behind
    /// it to switch.
    ///
    /// The `on` it names is the state being *asked for* and not the state it is in:
    /// an operation names a destination and never a toggle
    /// ([P-0090](../../../../docs/principles/0090-a-surface-offers-it-never-decides.md)),
    /// and the chip's toggle is this method.
    pub fn route(&self) -> Option<Operation> {
        self.present.then_some(Operation::RouteFrame {
            output: self.output,
            on: !self.on,
        })
    }

    /// Whether `p` is on this chip. Absent chips answer `false`: a control that
    /// switches nothing does not take a press away from the row it sits in.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.present && self.chip.contains(Pos2::new(p.x, p.y))
    }
}

/// The Outputs row, laid out: where the word goes, where the console's one
/// control is, and whether that control is lit.
///
/// # One derivation, because a control drawn where it cannot be clicked is
/// silent
///
/// [`View::draw`] paints exactly these rectangles and [`crate::input::claim`]
/// hit-tests exactly these rectangles, the way [`preview_cells`] serves both
/// [`preview_rects`] and the frame. Two copies of this arithmetic is a dot that
/// lights up under a pointer that cannot switch it, and nothing on screen says
/// so.
///
/// # `on` is read, never stored
///
/// The manual: *"The picture is a sink, listed in Outputs as program view, and
/// it is on screen exactly when that sink is on."* So the sink's state is
/// [`Layout::visible`] on the picture's node and there is no second copy of it
/// to drift — a fold from the keyboard lights the dot down, and the dot folds
/// the same node the keyboard does.
/// [ADR-0161](../../../../docs/adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md)
/// stored `soloed` because it could not be derived; this can.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Where the word OUTPUTS is painted: its top-left, and the box the galley
    /// fills.
    pub label: Rect,
    /// The class pill, beside the word that stands in for a head.
    ///
    /// This row is the one placement the console had no precedent for. The other
    /// three openings sit in a bay head, where [`head_pills`] has laid capsules out
    /// since the `solo` pill landed; this row is headless (ADR-0159,
    /// [`Kind::Outputs`]) — no hairline, no pills and no grip — so there was
    /// nothing to add a capsule to. `docs/manual/console.html` says where it goes
    /// and why in as many words: *"This row has no bay head to put an indicator in
    /// — it is headless, like the transport — so the pill sits beside the word that
    /// stands in for one."*
    ///
    /// So it is laid out here, in `.outputs`'s own flex row, between the word and
    /// the first sink: one [`size::OUTPUTS_GAP`] after the label, one before the
    /// chip, [`size::PILL_H`] tall and centred in the row like everything else in
    /// it. Which is why [`outputs`] takes an opening: the two words are not the
    /// same width, so where the sink starts depends on what the pill says.
    ///
    /// [`mcp_pill`] is what reads it back out, so that the four openings are one
    /// type and one probe however differently the two placements are arrived at.
    pub mcp: Rect,
    /// Whether the class this row's pill opens is open — read out of the opening
    /// handed in, and kept nowhere here.
    pub open: bool,
    /// The control: `.sink`'s capsule, dot and name together, which is what a press
    /// has to land in. The mock gives the whole chip the click, not the dot alone —
    /// a 7px dot is not a target a hand finds, which is [`GRAB`]'s argument one
    /// control along.
    pub sink: Rect,
    /// The `.dot` inside it.
    pub dot: Rect,
    /// The node this sink switches: the picture, `program-view`.
    pub id: NodeId,
    /// Whether the picture is on screen — `layout.visible(id)`, read here.
    pub on: bool,
    /// The rest of the list, left to right after the program view: the projector
    /// window, then the two plugin sinks.
    ///
    /// See [`SinkChip`] for why they are not one array with the program view in it.
    /// `on` is `false` on all three until [`Outputs::told`] says otherwise, and the
    /// two plugin chips are `present: false` for as long as there is no manifest to
    /// read them out of.
    pub more: [SinkChip; 3],
}

impl Outputs {
    /// What a press on the control asks for. Two operations and no toggle:
    /// [`Op::Fold`] while the picture is on, [`Op::Unfold`] while it is off. The
    /// toggle is this method — an affordance over two operations — and the
    /// vocabulary underneath it stays two things a MIDI map or an MCP call can ask
    /// for by name. See [`Op`].
    ///
    /// A press on a dark dot always lights it, whatever darkened it — the picture
    /// folded on its own, the Program bay folded around it, a solo that left it
    /// out. That is [`Op::Unfold`]'s rule and not a special case here: an unfold
    /// makes its node *visible*, so it undoes the way to it as well as the node.
    /// This control is why the rule is written that way, and the manual's own note
    /// on this row is the argument — *"Nothing is refused here, so nothing has to
    /// be explained: a control that quietly declines the last of something is a
    /// rule an operator can only find by experiment."* A press that lit nothing
    /// would be that rule with no words at all.
    pub fn op(&self) -> Op {
        match self.on {
            true => Op::Fold(self.id),
            false => Op::Unfold(self.id),
        }
    }

    /// What a press on the program view's chip asks for, in the vocabulary every
    /// surface shares.
    ///
    /// [`Outputs::op`] above is how the console *performs* it and this is what is
    /// *asked*, which is one fact and not two: the picture's on and off is the
    /// fold, so the operation names the output and the fold is what carries it out.
    /// Storing a second `on` beside the layout node is what this refuses —
    /// [ADR-0161](../../../../docs/adr/0161-solo-remembers-which-region-because-it-cannot-be-derived.md)
    /// stored `soloed` because it could not be derived; this can.
    pub fn route(&self) -> Operation {
        Operation::RouteFrame {
            output: Output::Program,
            on: !self.on,
        }
    }

    /// Say which of the outputs this crate cannot see are on.
    ///
    /// The picture's state is a layout node and this row reads it; a projector
    /// window is `crates/karakuri`'s and there is nothing in this crate that could
    /// know
    /// ([ADR-0156](../../../../docs/adr/0156-the-consoles-arrangement-is-a-tree-this-repository-owns.md)
    /// — the console takes no device). So it arrives the way [`View::picture`]
    /// does: written by whoever owns the window, beside the frame that draws it.
    ///
    /// A builder rather than a fourth argument to [`outputs`], because the answer
    /// is only needed to *paint* a chip and every caller that hit-tests one has no
    /// opinion about it.
    pub fn told(mut self, projector: bool) -> Outputs {
        self.more[0].on = projector;
        self
    }

    /// Whether `p` is on the program view's control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.sink.contains(Pos2::new(p.x, p.y))
    }

    /// Which chip `p` is on, over the whole row — the program view and the three
    /// beside it.
    ///
    /// `None` for a press on the row's ground, on the word, on the class pill or on
    /// a chip with nothing behind it. [`Outputs::hit`] is the same question asked
    /// of the first chip alone, and it stays because [`crate::input`]'s claim rule
    /// is written against one control.
    pub fn chip_at(&self, p: karakuri_layout::Point) -> Option<Output> {
        if self.hit(p) {
            return Some(Output::Program);
        }
        self.more.iter().find(|c| c.hit(p)).map(|c| c.output)
    }
}

/// The Outputs row's furniture, derived: the word, and the one sink.
///
/// `None` where there is no row to draw in — the row folded away, a solo
/// somewhere else, or a window too small to hold the chip — which is
/// [`picture_rect`]'s rule stated on a control instead of a picture: a
/// rectangle with nothing in it is not something to paint or to click.
///
/// # What is in the mock's row and is deliberately not here
///
/// The mock draws four `.sink`s and a `+ add output` pill. One of them
/// exists. Drawing the others is the scaffolding this module's
/// documentation refuses — a control that looks finished does not get
/// replaced, and each of these is a control over something that has not been
/// built:
///
/// - `projector · DELL U2720Q` is a second window on another display. That is
///   a second `Sink` and a second surface, and neither exists.
/// - `Syphon` and `NDI · no plugin` are plugin sinks. There is no plugin
///   system, and `NDI`'s own row says so — it is drawn `.absent`, which is a
///   control explaining that the thing it would switch is not installed.
/// - `+ add output` adds a region while the panel is running, which the arena
///   cannot do: `docs/roadmap.md` records it as one gap drawn five times
///   (`+ lane`, `+ add`, `+ add output`, `+` on the scope list, and the
///   inspector's `2 up`), and it arrives with the arena operations, not with
///   this row.
///
/// # The tooltip is not drawn, and not half-drawn either
///
/// Every `.sink` in the mock carries a `data-tip`, and the manual makes a
/// point of it: *"hover says three things: what the control is, what state it
/// is in, and what a click will do."* A tooltip needs `egui` to own a
/// widget — a `Response` with a hover state and a layer above the panel —
/// and this console paints, with no widget anywhere in it
/// ([`crate::input`]). Giving one control a widget is a decision about who
/// owns the pointer, and it is its own; so no tooltip is drawn here and no
/// half of one is left behind.
///
/// # The row does not wrap, and the mock's does
///
/// `.outputs` is `flex-wrap: wrap`, which is a rule about what happens to the
/// *fifth* thing in a row that is too narrow to hold it. There is one sink and
/// it fits at every width the console draws — 172 of a narrowest 990 — so
/// wrapping is a rule with nothing to apply to, and it is a decision to take
/// with the second sink rather than a behaviour to write for one.
///
/// `layout` must be solved: [`Layout::rect`] refuses to answer from a dirty
/// one. `ctx` is asked for the type: the chip's width is the width of the name
/// in it, and where the name goes is where the word before it ended.
pub fn outputs(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
) -> Option<Outputs> {
    // **Fonts are not valid until `egui` has run a pass**, and it says so
    // outright. A pointer event can reach this before the first frame — the
    // window is up and the loop has not drawn yet — so the answer there is
    // that there is no control, which is also the true one: a control that has
    // never been drawn is not one a press can be on.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("outputs")?));
    let id = layout.find("program-view")?;
    let label = ctx.fonts_mut(|f| f.layout_job(label_job(Color32::PLACEHOLDER)).size().x);
    // **The class pill's own word**, measured the way every capsule on this
    // console is: `.pill` is as wide as what is in it, and the two states are
    // not the same width.
    let opened = open.holds(Class::InputsAndOutputs);
    let pill = pill_width(ctx, mcp_word(opened));
    // **Every chip's name is measured, not just the first.** A capsule is as
    // wide as what is in it, so where the third starts depends on what the
    // second says — the same rule the class pill above made this row take for
    // its first chip, four times over.
    let widths: [f32; 4] = std::array::from_fn(|i| {
        let word = match i {
            0 => PROGRAM_VIEW,
            1 => PROJECTOR,
            n => PLUGIN_SINKS[n - 2],
        };
        ctx.fonts_mut(|f| {
            f.layout_no_wrap(
                word.to_owned(),
                FontId::new(size::BASE, FontFamily::Proportional),
                Color32::PLACEHOLDER,
            )
            .size()
            .x
        })
    });
    outputs_row(row, label, pill, widths).map(|OutputsRow { label, mcp, chips }| Outputs {
        label,
        mcp,
        open: opened,
        sink: chips[0].0,
        dot: chips[0].1,
        id,
        on: layout.visible(id),
        more: std::array::from_fn(|i| SinkChip {
            output: match i {
                0 => Output::Projector(0),
                n => Output::Plugin(n as u8 - 1),
            },
            chip: chips[i + 1].0,
            dot: chips[i + 1].1,
            on: false,
            present: i == 0,
            name: match i {
                0 => PROJECTOR,
                n => PLUGIN_SINKS[n - 1],
            },
        }),
    })
}

/// The arithmetic of the row, away from the type it measures and the layout it
/// reads — the two rectangles [`outputs`] hands out and the dot inside the
/// second.
///
/// Term for term from `.outputs` and `.sink` in `style.css`:
///
/// - `.outputs { display: flex; align-items: center; gap: 8px;
///   padding: 8px 11px }` — the word and the chip laid left to right from
///   [`size::OUTPUTS_PAD_X`], one [`size::OUTPUTS_GAP`] between them, and both
///   centred in the row rather than sat on its padding.
/// - `.sink { padding: 1px 10px; gap: 6px; border-radius: 999px }` — a
///   [`size::SINK_H`] capsule holding a [`size::SINK_DOT`] dot, a
///   [`size::SINK_GAP`], and the name.
///
/// The centring is where the 34 comes back. The row is 34 and a sink is
/// 18.5, so there is (34 - 18.5) / 2 = 7.75 of row above the chip and 7.75
/// below — more than the six pixels [`GRAB`] widens the boundary above it by,
/// which is the whole reason this control can be clicked at all. `.outputs`'s
/// own padding is 8 and the arrangement rounded 8 + 18.5 + 8 down to 34, so
/// the quarter pixel the CSS and the row disagree by is spent here rather than
/// argued about: `align-items: center` is what the CSS says, and it is what
/// leaves the two clearances equal.
/// What [`outputs_row`] hands back: the word, the class pill and the four
/// chips, each with the dot inside it.
///
/// A named type rather than a tuple because the tuple grew a term per chip and
/// stopped being readable at the call site — which is the same reason
/// [`Outputs`] itself is a struct.
struct OutputsRow {
    label: Rect,
    mcp: Rect,
    /// The capsule and its dot, per chip, left to right.
    chips: [(Rect, Rect); 4],
}

fn outputs_row(row: Rect, label_w: f32, pill_w: f32, name_w: [f32; 4]) -> Option<OutputsRow> {
    let mid = row.center().y;
    let label = Rect::from_min_size(
        Pos2::new(row.min.x + size::OUTPUTS_PAD_X, mid - size::HEAD_SIZE * 0.5),
        egui::vec2(label_w, size::HEAD_SIZE),
    );
    // **The class pill, between the word and the first sink**, which is where
    // `docs/manual/console.html` draws it and why: this row has no head to put
    // an indicator in, so it sits beside the word that stands in for one. It is
    // a `.pill` and not a `.sink` — [`size::PILL_H`] and not
    // [`size::SINK_H`] — because it is the same capsule the three bay heads
    // draw, and a control that looked like a sink here would read as a fifth
    // output.
    let mcp = Rect::from_min_size(
        Pos2::new(label.max.x + size::OUTPUTS_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // **The chips, left to right from the pill, one `.outputs` gap apart.**
    let mut x = mcp.max.x;
    let chips: [(Rect, Rect); 4] = std::array::from_fn(|i| {
        let chip = Rect::from_min_size(
            Pos2::new(x + size::OUTPUTS_GAP, mid - size::SINK_H * 0.5),
            egui::vec2(
                size::SINK_PAD_X * 2.0 + size::SINK_DOT + size::SINK_GAP + name_w[i],
                size::SINK_H,
            ),
        );
        x = chip.max.x;
        let dot = Rect::from_center_size(
            Pos2::new(chip.min.x + size::SINK_PAD_X + size::SINK_DOT * 0.5, mid),
            egui::vec2(size::SINK_DOT, size::SINK_DOT),
        );
        (chip, dot)
    });
    // The same rule [`picture_rect`] states, on the chips rather than on the
    // row because a chip is what is drawn and clicked: a row folded away has
    // a rectangle with no extent in it, and one too narrow for the chips has
    // nowhere to put them. Either way there is no control.
    //
    // **It is the whole list or none of it, and the mock's `flex-wrap: wrap`
    // is still a rule with nothing to apply to.** The row is one
    // [`size::OUTPUTS_H`] tall in an arrangement that cannot grow it, so
    // wrapping is not available; and the four chips, the word and the class
    // pill come to well under the narrowest console this draws, so the case
    // where they do not fit is the same case the row is folded away in.
    // Dropping the tail silently would be the worse answer of the two — a
    // control that is not drawn cannot say it is not drawn — and it is not
    // reachable either.
    match row.contains_rect(chips[3].0) {
        true => Some(OutputsRow { label, mcp, chips }),
        false => None,
    }
}

/// The word OUTPUTS as one laid-out run, so that measuring it and painting it
/// cannot be two different runs of type.
fn label_job(colour: Color32) -> LayoutJob {
    spaced(
        &OUTPUTS_LABEL.to_uppercase(),
        size::HEAD_SIZE,
        colour,
        size::HEAD_TRACKING,
    )
}

/// The Outputs row's contents: the word, and the one `.sink`.
///
/// Where everything goes is [`outputs`]'s, so this paints and derives nothing.
/// Term for term from `style.css`:
///
/// - `.sink` — `background: var(--c-well)`, `color: var(--c-dim)`, and
///   `border-radius: 999px`, which on a box this short is a capsule.
/// - `.sink.on` — `color: var(--c-text)` over
///   `color-mix(in srgb, var(--c-mint) 14%, transparent)`, which is
///   `pal.mint` at 14% alpha exactly (see `room`'s documentation).
/// - `.dot` — `var(--c-faint)` off, and `var(--c-mint)` with
///   `box-shadow: 0 0 8px var(--c-glow)` on. The halo is an
///   [`egui::epaint::Shadow`] with the blur the CSS names and a corner radius
///   of half the dot, which is a blurred circle — the same mechanism the bay's
///   card draws its drop shadow with, and the first thing in this crate to use
///   `--c-glow`, which `room` transcribed against the day a control was drawn.
pub(super) fn outputs_into(ui: &Ui, pal: &Palette, row: &Outputs) {
    let painter = ui.painter();

    // The word: `.bay-head`'s type on a row that has no head — see
    // `OUTPUTS_LABEL`.
    let galley = painter.layout_job(label_job(pal.faint));
    painter.galley(row.label.min, galley, pal.faint);

    // **The class pill, painted exactly as a bay head's is** — the same
    // [`pill_at`] the other three go through, in a row that has no head to put
    // it in. See [`Outputs::mcp`].
    pill_into(ui, pal, row.mcp, mcp_word(row.open), row.open);

    // **The program view first, then the three beside it**, all four through
    // one closure: three states and one set of colours, so a chip cannot be
    // painted one way here and another way one line down.
    //
    // `.sink` is `--c-well` with `--c-dim` type; `.sink.on` is the mint tint,
    // `--c-text` and the dot's glow; and `.sink.absent` is the well with the
    // faintest type there is and no glow, which is the mock's own state for a
    // sink whose plugin is not loaded — dim enough to read as *not installed*
    // rather than as *switched off*.
    let chip_into = |chip: Rect, dot_at: Rect, name: &str, on: bool, present: bool| {
        let (ink, fill, dot) = match (present, on) {
            (false, _) => (pal.faint, pal.well, tint(pal.faint, 40)),
            (true, true) => (pal.text, tint(pal.mint, 14), pal.mint),
            (true, false) => (pal.dim, pal.well, pal.faint),
        };
        painter.rect_filled(chip, CornerRadius::same((size::SINK_H * 0.5) as u8), fill);
        if on && present {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: 8,
                    spread: 0,
                    color: pal.glow,
                }
                .as_shape(dot_at, CornerRadius::same((size::SINK_DOT * 0.5) as u8)),
            );
        }
        painter.circle_filled(dot_at.center(), size::SINK_DOT * 0.5, dot);
        let galley = painter.layout_no_wrap(
            name.to_owned(),
            FontId::new(size::BASE, FontFamily::Proportional),
            ink,
        );
        painter.galley(
            Pos2::new(
                dot_at.max.x + size::SINK_GAP,
                chip.center().y - galley.size().y * 0.5,
            ),
            galley,
            ink,
        );
    };
    chip_into(row.sink, row.dot, PROGRAM_VIEW, row.on, true);
    for c in &row.more {
        chip_into(c.chip, c.dot, c.name, c.on, c.present);
    }
}
