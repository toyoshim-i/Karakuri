use super::*;

// ---------------------------------------------------------------------------
// The Outputs row
// ---------------------------------------------------------------------------

/// Header label for the Outputs row (ADR-0159).
const OUTPUTS_LABEL: &str = "Outputs";

/// The one sink the console has, and the manual's name for it: *"The picture in
/// the Program bay is a sink like any other and is the first row."*
const PROGRAM_VIEW: &str = "program view";

/// Projector output sink label.
const PROJECTOR: &str = "projector";

/// Default labels for plugin sinks when no plugin provider is loaded.
#[cfg(target_os = "macos")]
const PLUGIN_0_NAME: &str = "Syphon";
#[cfg(not(target_os = "macos"))]
const PLUGIN_0_NAME: &str = "Spout";

#[cfg(target_os = "macos")]
const PLUGIN_0_ABSENT: &str = "Syphon · no plugin";
#[cfg(not(target_os = "macos"))]
const PLUGIN_0_ABSENT: &str = "Spout · no plugin";

const PLUGIN_SINKS: [&str; 2] = [PLUGIN_0_ABSENT, "NDI · no plugin"];

/// Represents an auxiliary output sink chip (projector or plugin stream).
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
    /// Returns the frame routing operation to toggle this sink, or `None` if absent (P-0090).
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

/// Layout and display geometry for the Outputs row.
///
/// Sink status is derived directly from layout node visibility (ADR-0161).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outputs {
    /// Where the word OUTPUTS is painted: its top-left, and the box the galley
    /// fills.
    pub label: Rect,
    /// Bounding box for the MCP class pill beside the label.
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
    /// Auxiliary sink chips (projector and plugin sinks) in left-to-right order.
    pub more: [SinkChip; 3],
}

impl Outputs {
    /// Returns the toggle operation ([`Op::Fold`] or [`Op::Unfold`]) for the program view.
    pub fn op(&self) -> Op {
        match self.on {
            true => Op::Fold(self.id),
            false => Op::Unfold(self.id),
        }
    }

    /// Returns the frame routing operation to toggle the program view (ADR-0161).
    pub fn route(&self) -> Operation {
        Operation::RouteFrame {
            output: Output::Program,
            on: !self.on,
        }
    }

    /// Configures external projector state on the first auxiliary sink (ADR-0156).
    pub fn told(mut self, projector: bool) -> Outputs {
        self.more[0].on = projector;
        self
    }

    /// Say whether an output plugin sink is active and whether it is available on this machine.
    pub fn told_plugin(self, index: usize, on: bool, present: bool) -> Outputs {
        self.told_plugin_name(index, on, present, None)
    }

    /// Say whether an output plugin sink is active, available, and its custom display name.
    pub fn told_plugin_name(
        mut self,
        index: usize,
        on: bool,
        present: bool,
        name: Option<&'static str>,
    ) -> Outputs {
        if let Some(chip) = self.more.get_mut(index + 1) {
            chip.on = on;
            chip.present = present;
            if index == 0 {
                chip.name = match (present, name) {
                    (true, Some(n)) => n,
                    (true, None) => PLUGIN_0_NAME,
                    (false, _) => PLUGIN_SINKS[0],
                };
            }
        }
        self
    }

    /// Whether `p` is on the program view's control.
    pub fn hit(&self, p: karakuri_layout::Point) -> bool {
        self.sink.contains(Pos2::new(p.x, p.y))
    }

    /// Hit-tests point `p` against all active sink chips in the row.
    pub fn chip_at(&self, p: karakuri_layout::Point) -> Option<Output> {
        if self.hit(p) {
            return Some(Output::Program);
        }
        self.more.iter().find(|c| c.hit(p)).map(|c| c.output)
    }
}

/// Computes layout geometry for the Outputs row and its sinks.
/// Returns `None` if the row is folded, off-screen, or before fonts initialize.
pub fn outputs(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
) -> Option<Outputs> {
    outputs_with(ctx, layout, open, false)
}

/// The Outputs row, laid out: where the word goes, where the console's one
/// control is, and whether that control is lit. Allows specifying whether plugin
/// sink 0 (Syphon) is present on this machine.
pub fn outputs_with(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
    plugin_available: bool,
) -> Option<Outputs> {
    outputs_with_plugin_name(ctx, layout, open, plugin_available, None)
}

/// The Outputs row, laid out: where the word goes, where the console's one
/// control is, and whether that control is lit. Allows specifying whether plugin
/// sink 0 is present on this machine and an optional custom plugin display name.
pub fn outputs_with_plugin_name(
    ctx: &egui::Context,
    layout: &karakuri_layout::Layout,
    open: Open,
    plugin_available: bool,
    plugin_name: Option<&'static str>,
) -> Option<Outputs> {
    // Defer layout until egui completes its initial pass and fonts become valid.
    if ctx.cumulative_pass_nr() == 0 {
        return None;
    }
    let row = to_egui(layout.rect(layout.find("outputs")?));
    let id = layout.find("program-view")?;
    let label = ctx.fonts_mut(|f| f.layout_job(label_job(Color32::PLACEHOLDER)).size().x);
    // Measure class pill based on open state.
    let opened = open.holds(Class::InputsAndOutputs);
    let pill = pill_width(ctx, mcp_word(opened));

    let custom_0_name = plugin_name.unwrap_or(PLUGIN_0_NAME);

    // Measure label widths for all four sink chips.
    let widths: [f32; 4] = std::array::from_fn(|i| {
        let word = match i {
            0 => PROGRAM_VIEW,
            1 => PROJECTOR,
            2 => {
                if plugin_available {
                    custom_0_name
                } else {
                    PLUGIN_SINKS[0]
                }
            }
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
            present: match i {
                0 => true,
                1 => plugin_available,
                _ => false,
            },
            name: match i {
                0 => PROJECTOR,
                1 => {
                    if plugin_available {
                        custom_0_name
                    } else {
                        PLUGIN_SINKS[0]
                    }
                }
                n => PLUGIN_SINKS[n - 1],
            },
        }),
    })
}

/// Layout results for the outputs row: label, MCP class pill, and sink chips with indicator dots.
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
    // Class pill positioned between the label and first sink (PILL_H height).
    let mcp = Rect::from_min_size(
        Pos2::new(label.max.x + size::OUTPUTS_GAP, mid - size::PILL_H * 0.5),
        egui::vec2(pill_w, size::PILL_H),
    );
    // Layout chips horizontally left-to-right from the MCP pill.
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
    // Require all chips to fit within the row bounds without wrapping.
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

/// Renders Outputs row contents: section label, MCP pill, and sink chips.
pub(super) fn outputs_into(ui: &Ui, pal: &Palette, row: &Outputs) {
    let painter = ui.painter();

    // The word: `.bay-head`'s type on a row that has no head — see
    // `OUTPUTS_LABEL`.
    let galley = painter.layout_job(label_job(pal.faint));
    painter.galley(row.label.min, galley, pal.faint);

    // Renders the class pill matching bay head style.
    pill_into(ui, pal, row.mcp, mcp_word(row.open), row.open);

    // Renders program view and auxiliary sink chips according to state (present, on, absent).
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
