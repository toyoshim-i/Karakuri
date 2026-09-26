use super::super::*;
use egui::epaint::text::{FontId, LayoutJob, TextFormat};

// ---------------------------------------------------------------------------
// Bay head rendering, capsule layout, and head metadata
// ---------------------------------------------------------------------------

/// Furniture for a bay head: title, controls, grip indicator, and opened class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Head {
    /// The mock's own capitalisation; `.bay-head` upper-cases at paint time.
    pub title: &'static str,
    /// The controls the table lists in this head, and only the controls.
    pub pills: &'static [&'static str],
    /// Whether the mock draws a `.grip` at the right of it.
    pub grip: bool,
    /// The class this head opens, where it opens one — [`class_at`].
    pub class: Option<Class>,
    /// Active bank pill index (0..3) if this head displays bank pills.
    pub banks: Option<usize>,
    /// Whether the head indicates an in-flight background build.
    pub building: bool,
}

/// Returns the [`Head`] descriptor for `region`, or `None` if headless (e.g. Transport, Outputs per ADR-0159).
pub fn head_of(region: &Region) -> Option<Head> {
    let (title, pills): (_, &'static [&'static str]) = match region.kind {
        Kind::Bay { title, pills, .. } => (title, pills),
        Kind::Mixer => (mixer::MIXER_TITLE, &[]),
        Kind::Master => (master::MASTER_TITLE, &[]),
        Kind::Library => (library::LIBRARY_TITLE, &[]),
        Kind::Staging => (staging::STAGING_TITLE, &[]),
        Kind::Sequencer => (sequencer::SEQUENCER_TITLE, &[]),
        Kind::Prompt => (prompt::PROMPT_TITLE, &[]),
        Kind::Transport | Kind::Outputs | Kind::Pane | Kind::Picture | Kind::Previews => {
            return None
        }
    };
    Some(Head {
        title,
        pills,
        grip: head_grip(region.kind),
        class: class_at(region.name),
        banks: None,
        building: false,
    })
}

/// Returns whether the bay of `kind` renders a fold grip.
pub(crate) const fn head_grip(kind: Kind) -> bool {
    match kind {
        Kind::Bay { grip, .. } => grip,
        Kind::Library
        | Kind::Master
        | Kind::Mixer
        | Kind::Staging
        | Kind::Sequencer
        | Kind::Prompt => true,
        Kind::Transport | Kind::Outputs | Kind::Pane | Kind::Picture | Kind::Previews => false,
    }
}

/// Total number of bay heads that draw a grip across [`REGIONS`].
pub const BAY_GRIPS: usize = grips();

/// [`BAY_GRIPS`], in a `const` — which `Iterator::filter` is not.
const fn grips() -> usize {
    let mut total = 0;
    let mut at = 0;
    while at < REGIONS.len() {
        if head_grip(REGIONS[at].kind) {
            total += 1;
        }
        at += 1;
    }
    total
}

/// Maximum capsules a bay head can hold: controls, class pill, building indicator, and bank pills.
const HEAD_PILLS: usize = 3 + karakuri_pattern::BANKS;

/// And every entry fits with everything a head can be handed beside it — the
/// class pill, the building indicator, and the four bank pills. A head listing one more control than
/// that would lose a capsule off the end of [`HeadWords`] silently, which is a
/// control that stops existing rather than a build that stops.
const _: () = {
    let mut at = 0;
    while at < REGIONS.len() {
        if let Kind::Bay { pills, .. } = REGIONS[at].kind {
            assert!(pills.len() + 2 + karakuri_pattern::BANKS <= HEAD_PILLS)
        }
        at += 1;
    }
};

/// Capsules displayed in a bay head for the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadWords {
    words: [&'static str; HEAD_PILLS],
    /// Which of them are lit, in the same order — see [`HeadWords::armed`].
    armed: [bool; HEAD_PILLS],
    len: usize,
}

impl HeadWords {
    /// The words, in the order [`head_pills`] takes them.
    pub fn as_slice(&self) -> &[&'static str] {
        &self.words[..self.len]
    }

    /// Returns whether the capsule at `index` is drawn in the armed state.
    pub fn armed(&self, index: usize) -> bool {
        self.armed.get(index).copied().unwrap_or(false)
    }
}

impl Head {
    /// This head with `armed` of its four bank pills lit — the one door into
    /// [`Head::banks`], so a head that draws banks is a head somebody handed a
    /// value to.
    pub fn with_banks(self, armed: usize) -> Head {
        Head {
            banks: Some(armed),
            ..self
        }
    }

    /// This head with the building indicator badge enabled or disabled.
    pub fn with_building(self, building: bool) -> Head {
        Head { building, ..self }
    }

    /// This head's capsules under `open` — see [`HeadWords`].
    pub fn words(&self, open: Open) -> HeadWords {
        let mut words = [""; HEAD_PILLS];
        let mut armed = [false; HEAD_PILLS];
        let mut len = 0;
        for pill in self.pills.iter().take(HEAD_PILLS) {
            words[len] = pill;
            len += 1;
        }
        // Bank capsules sit between table controls and the class pill, keeping class rightmost.
        if let Some(at) = self.banks {
            for (bank, word) in BANK_PILLS.iter().enumerate() {
                words[len] = word;
                armed[len] = bank == at;
                len += 1;
            }
        }
        if self.building {
            words[len] = "building";
            armed[len] = true;
            len += 1;
        }
        if let Some(class) = self.class {
            words[len] = mcp_word(open.holds(class));
            armed[len] = open.holds(class);
            len += 1;
        }
        HeadWords { words, armed, len }
    }
}

/// Labels for bank pills (`seq 1`..`seq 4`).
const BANK_PILLS: [&str; karakuri_pattern::BANKS] = ["seq 1", "seq 2", "seq 3", "seq 4"];

/// Finds the bounding rectangle of a capsule in `head` matching `want`.
pub(crate) fn head_capsule(
    ctx: &egui::Context,
    rect: Rect,
    head: &Head,
    open: Open,
    want: &str,
) -> Option<Rect> {
    let words = head.words(open);
    let words = words.as_slice();
    let mut found = None;
    head_pills(ctx, rect, words, head.grip, |index, capsule| {
        if words[index] == want {
            found = Some(capsule);
        }
    });
    found.filter(|capsule| head_box(rect).contains_rect(*capsule))
}

/// Bounding rectangles for each bank pill in bank order, or empty if head cannot fit all four.
pub(crate) fn bank_capsules(ctx: &egui::Context, rect: Rect, head: &Head, open: Open) -> Vec<Rect> {
    if head.banks.is_none() {
        return Vec::new();
    }
    let words = head.words(open);
    let slice = words.as_slice();
    // Where the banks start: the table's own controls come first, and this
    // head has none of them today — read off the same walk rather than
    // assumed, so a Sequencer head that ever lists a control still finds them.
    let first = head.pills.len().min(slice.len());
    let box_of = head_box(rect);
    let mut out = vec![None; karakuri_pattern::BANKS];
    head_pills(ctx, rect, slice, head.grip, |index, capsule| {
        if let Some(bank) = index.checked_sub(first) {
            if let Some(slot) = out.get_mut(bank) {
                *slot = Some(capsule).filter(|it| box_of.contains_rect(*it));
            }
        }
    });
    match out.iter().all(Option::is_some) {
        true => out.into_iter().flatten().collect(),
        false => Vec::new(),
    }
}

/// Paints a folded bay's header showing its title and open shortcut hint.
pub(crate) fn folded_head_into(ui: &Ui, pal: &Palette, at: Rect, title: &str) {
    bay_card(ui, pal, at);
    let painter = ui.painter().with_clip_rect(at);
    let mid = at.center().y;
    let job = spaced(
        &title.to_uppercase(),
        size::HEAD_SIZE,
        pal.faint,
        size::HEAD_TRACKING,
    );
    let galley = painter.layout_job(job);
    let left = at.min.x + size::HEAD_PAD_X;
    painter.galley(
        Pos2::new(left, mid - galley.size().y * 0.5),
        galley.clone(),
        pal.faint,
    );
    // Shortcut hint placed immediately after title rather than right-aligned.
    let says = painter.layout_no_wrap(
        focus::OPENS.to_owned(),
        FontId::new(size::HEAD_SIZE, FontFamily::Monospace),
        pal.faint,
    );
    let after = match title.is_empty() {
        true => left,
        false => left + galley.size().x + size::HEAD_PAD_X,
    };
    painter.galley(Pos2::new(after, mid - says.size().y * 0.5), says, pal.faint);
}

/// Paints a bay head with title on the left and controls/grip on the right.
pub fn bay_head(ui: &Ui, pal: &Palette, rect: Rect, head: &Head, open: Open) -> Rect {
    let box_of = head_box(rect);
    let painter = ui.painter().with_clip_rect(box_of);

    // `border-bottom: 1px solid var(--c-hair)`.
    let rule = box_of.max.y - size::HAIRLINE * 0.5;
    painter.line_segment(
        [Pos2::new(box_of.min.x, rule), Pos2::new(box_of.max.x, rule)],
        Stroke::new(size::HAIRLINE, pal.hair),
    );

    let mid = box_of.center().y;
    if head.grip {
        grip_dots(ui, pal, Pos2::new(box_of.max.x - size::HEAD_PAD_X, mid));
    }
    // Painted at positions computed by head_pills for unified visual and hit-test geometry.
    let armed = head.words(open);
    let words = armed.as_slice();
    head_pills(ui.ctx(), rect, words, head.grip, |index, capsule| {
        pill_into(ui, pal, capsule, words[index], armed.armed(index));
    });

    // `text-transform: uppercase` plus `letter-spacing: 0.16em`, at
    // `--c-faint`. `egui`'s default proportional face has no bold, so
    // `font-weight: 700` is not honoured — see `room`'s documentation.
    let job = spaced(
        &head.title.to_uppercase(),
        size::HEAD_SIZE,
        pal.faint,
        size::HEAD_TRACKING,
    );
    let galley = painter.layout_job(job);
    painter.galley(
        Pos2::new(box_of.min.x + size::HEAD_PAD_X, mid - galley.size().y * 0.5),
        galley,
        pal.faint,
    );

    box_of
}

/// Paints the header for `region` if it has one.
pub(crate) fn head_into(ui: &Ui, pal: &Palette, rect: Rect, region: &Region, open: Open) {
    if let Some(head) = head_of(region) {
        bay_head(ui, pal, rect, &head, open);
    }
}

/// Bounding box for a bay head within `rect`, clipped to the bay's height.
pub(crate) fn head_box(rect: Rect) -> Rect {
    Rect::from_min_max(
        rect.min,
        Pos2::new(rect.max.x, (rect.min.y + size::HEAD_H).min(rect.max.y)),
    )
}

/// A run of text with `letter-spacing`, which `egui` states per format run
/// rather than per style.
pub(crate) fn spaced(text: &str, size: f32, colour: Color32, tracking: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.append(
        text,
        0.0,
        TextFormat {
            font_id: FontId::new(size, FontFamily::Proportional),
            extra_letter_spacing: tracking,
            color: colour,
            ..Default::default()
        },
    );
    job
}
