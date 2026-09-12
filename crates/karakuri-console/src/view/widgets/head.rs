use super::super::*;
use egui::epaint::text::{FontId, LayoutJob, TextFormat};

// ---------------------------------------------------------------------------
// Bay head rendering, capsule layout, and head metadata
// ---------------------------------------------------------------------------

/// A bay head's furniture: the word in it, the controls the mock draws in it,
/// whether it carries a grip, and the class it opens.
///
/// [`Kind::Bay`] carries the first three in [`REGIONS`] and the four bays that
/// are kinds of their own — the Mixer, the Master, the Library and the Staging
/// lane — carried them nowhere, because [`View::draw`] wrote them out at its
/// own call to [`bay_head`]. That was one copy while nothing but the paint
/// needed them and it is four copies now: [`mcp_pill`] has to lay a head's
/// pills out again to find the class capsule among them, and a head whose words
/// the paint and the press disagreed about is precisely the defect
/// [`head_pills`] was written against.
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
    /// The bank pills, and which of the four is armed — `None` for every head but
    /// the Sequencer's, which is the one head in [`REGIONS`] whose capsules are a
    /// *value* rather than a table entry.
    ///
    /// # Why the head machinery grew a field rather than the table growing a row
    ///
    /// [`Head::pills`] is `&'static [&'static str]`, which is every capsule this
    /// console had until 2026-09-09: a word chosen when the region was written. The
    /// Sequencer's `seq 1 … seq 4` are four words that are always the same and one
    /// mark that is not — which bank the lanes are reading — and
    /// [`view::sequencer`](sequencer)'s own note named that as the missing piece:
    /// *"what is missing is a head that can say which pill is armed"*.
    ///
    /// It is [`mcp_pill`]'s arrangement and not a second one. A class pill carries
    /// a state into a head already; it does it by [`mcp_word`], because its two
    /// states have two words. A bank's do not — `seq 2` reads `seq 2` whether it is
    /// armed or not — so what a bank needs is the other half of what the class pill
    /// already gets: [`HeadWords::armed`], which is where *is this capsule lit* is
    /// answered for both of them now.
    ///
    /// Every other head is `None` and its [`HeadWords`] is what it was, to the word
    /// and to the bit; `tests/head_words.rs` renders every region's head before and
    /// after and compares.
    pub banks: Option<usize>,
}

/// The head a region draws, or `None` where it has none.
///
/// The transport and the Outputs row are headless (ADR-0159), and a pane, the
/// picture and the preview row are inside a bay that has one. The Outputs row
/// answering `None` here is what makes the fourth class pill a placement of its
/// own — see [`outputs`], which is where it goes instead.
pub fn head_of(region: &Region) -> Option<Head> {
    let (title, pills): (_, &'static [&'static str]) = match region.kind {
        Kind::Bay { title, pills, .. } => (title, pills),
        Kind::Mixer => (mixer::MIXER_TITLE, &[]),
        Kind::Master => (master::MASTER_TITLE, &[]),
        Kind::Library => (library::LIBRARY_TITLE, &[]),
        Kind::Staging => (staging::STAGING_TITLE, &[]),
        Kind::Sequencer => (sequencer::SEQUENCER_TITLE, &[]),
        Kind::Transport | Kind::Outputs | Kind::Pane | Kind::Picture | Kind::Previews => {
            return None
        }
    };
    Some(Head {
        title,
        pills,
        grip: head_grip(region.kind),
        class: class_at(region.name),
        // **No head is born with banks**, which is what keeps this table a
        // table: the armed bank is a value the host writes per frame, so the
        // one head that draws them asks for them — [`Head::with_banks`].
        banks: None,
    })
}

/// Whether a region's head draws a grip, said once and in a form a `const` can
/// ask.
///
/// It was the third term of [`head_of`]'s own match, which was one statement
/// while the paint was the only reader of it. [`BAY_GRIPS`] is the second
/// reader and it is a `const`: [`crate::input`]'s table says how many controls
/// a probe reaches, that number is *how many heads draw one of these*, and a
/// hand-written four is the shape of thing this crate counts rather than
/// remembers ([`REGIONS`], `Scope::ALL`, `Class::ALL`). A `bool` per kind in
/// two places would be two answers to *does this head carry a grip*, and the
/// day they disagreed the mark and the control would be in different heads.
///
/// `Kind::Bay` carries its own, because [`REGIONS`] states it there; the four
/// bays that are kinds of their own state it here, which is where they stated
/// it before. Everything headless answers `false` rather than being
/// unreachable, so the count above is a walk over every region and not over a
/// subset somebody has to keep.
pub(crate) const fn head_grip(kind: Kind) -> bool {
    match kind {
        Kind::Bay { grip, .. } => grip,
        // The mock draws one in these two heads and not in the other two —
        // see each kind's own documentation, which is where the reading is.
        Kind::Library | Kind::Master => true,
        Kind::Mixer | Kind::Staging | Kind::Sequencer => false,
        Kind::Transport | Kind::Outputs | Kind::Pane | Kind::Picture | Kind::Previews => false,
    }
}

/// How many bay heads draw a grip, counted off [`REGIONS`] — which is how many
/// bays a pointer can fold, because the grip is the control ([`bay_grip`]).
///
/// Four, on the day it is written: the Library, the Program bay, the Inspector
/// and the Master. It is a count and not a four for [`crate::input::PROBES`]'
/// reason — the row that registers this control says how many controls it
/// reaches, and a grip drawn in a fifth head has to raise that number on its
/// own rather than wait for somebody to notice.
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

/// How many capsules one bay head can hold: the most any entry in [`REGIONS`]
/// lists — the Program bay's `solo` — plus the class pill, plus the bank pills
/// the Sequencer's head draws off a value ([`Head::banks`]).
///
/// The four are added rather than maxed, which is deliberately the pessimistic
/// reading: nothing says a head with banks may not also list a control and open
/// a class, and a bound that assumed otherwise would drop a capsule off the end
/// of [`HeadWords`] in silence the day one did — which is exactly what the
/// `const` assertion below exists to stop.
const HEAD_PILLS: usize = 2 + karakuri_pattern::BANKS;

/// And every entry fits with everything a head can be handed beside it — the
/// class pill and the four bank pills. A head listing one more control than
/// that would lose a capsule off the end of [`HeadWords`] silently, which is a
/// control that stops existing rather than a build that stops.
const _: () = {
    let mut at = 0;
    while at < REGIONS.len() {
        if let Kind::Bay { pills, .. } = REGIONS[at].kind {
            assert!(pills.len() + 1 + karakuri_pattern::BANKS <= HEAD_PILLS)
        }
        at += 1;
    }
};

/// Every capsule in a head this frame: the table's own controls, and then the
/// class pill where the bay opens a class.
///
/// The class pill is last, which is rightmost. [`head_pills`] lays a head out
/// right to left, and `docs/manual/console.html` draws the Program bay's head
/// as `1920×1080`, `solo`, `mcp · shut`, `previews 3 of 4` — so the opening
/// sits to the right of `solo`. That is read off the page rather than chosen
/// here, and it is why `solo`'s own capsule moves when a class is opened: the
/// two words are not the same width, and one derivation answering for both is
/// what keeps the pill an operator sees and the pill a press lands on the same
/// rectangle.
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

    /// Whether the capsule at `index` is drawn `.pill.armed`, which is one question
    /// with two answers behind it and used to be asked at the paint.
    ///
    /// [`bay_head`] read `words[index] == MCP_OPEN`, which works only while *lit*
    /// and *this word* are the same fact. They are for a class pill, whose two
    /// states are two words ([`mcp_word`]); they are not for a bank pill, which
    /// reads `seq 2` armed and `seq 2` plain. So the bit is derived where the word
    /// is, beside it, and the paint reads one answer instead of re-deriving one of
    /// them.
    ///
    /// `false` past the end, which is a capsule that is not there.
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

    /// This head's capsules under `open` — see [`HeadWords`].
    pub fn words(&self, open: Open) -> HeadWords {
        let mut words = [""; HEAD_PILLS];
        let mut armed = [false; HEAD_PILLS];
        let mut len = 0;
        for pill in self.pills.iter().take(HEAD_PILLS) {
            words[len] = pill;
            len += 1;
        }
        // **The banks sit between the table's controls and the class pill**,
        // which keeps the class pill rightmost — the rule this type's own
        // documentation states and reads off the mock.
        if let Some(at) = self.banks {
            for (bank, word) in BANK_PILLS.iter().enumerate() {
                words[len] = word;
                armed[len] = bank == at;
                len += 1;
            }
        }
        if let Some(class) = self.class {
            words[len] = mcp_word(open.holds(class));
            armed[len] = open.holds(class);
            len += 1;
        }
        HeadWords { words, armed, len }
    }
}

/// The bank pills' words, one per `karakuri_pattern::BANKS` — `seq 1` … `seq
/// 4`, counting from one as everything an operator reads on this console does.
///
/// A `const` array sized off that crate's own count, so a fifth bank is a
/// missing word here rather than a pill nobody draws.
const BANK_PILLS: [&str; karakuri_pattern::BANKS] = ["seq 1", "seq 2", "seq 3", "seq 4"];

/// One capsule in a bay head, by the word in it — [`head_pills`]'s answer,
/// asked for one pill rather than for all of them.
///
/// The one derivation three readers share: [`bay_head`] paints from it,
/// [`program_head`] finds `solo` in it and [`mcp_pill`] finds the class pill in
/// it. `None` where the head is too short to hold the capsule, which is a bay
/// clipped shorter than its own head — a capsule half out of a head is not one
/// to press.
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

/// Where each of a head's bank pills is, in bank order — [`head_capsule`] asked
/// by position instead of by word, because four capsules are wanted and not
/// one.
///
/// By index and not by word, which is the difference that matters: the four
/// bank words are distinct today, and a control found by the word in it is a
/// control that moves the day two capsules read the same thing. The class pill
/// can be found by its word because [`mcp_word`]'s two are the state; a bank's
/// is not ([`HeadWords::armed`]).
///
/// Empty for a head that draws no banks, and empty for one that cannot hold all
/// four — a capsule half out of a head is not one to press ([`head_capsule`]'s
/// own filter), and dropping the ones that fit would renumber the rest: the
/// pills are laid out right to left, so the capsule that falls off is `seq 1`
/// and every bank after it would answer for its neighbour.
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

/// A folded bay's mark, painted — the head alone, and the whole of what a
/// folded bay draws.
///
/// `docs/manual/console.html` is the specification, term for term: a
/// `.bay-head` with `border-bottom: 0`, the bay's title in the head's own
/// spaced-out capitals, and [`focus::OPENS`] beside it in the code face at
/// `--c-faint`. The dashed ring is the caller's, so this paints a head and
/// nothing else.
///
/// It says there is a bay here and that `space` opens it, and nothing else —
/// not a bay's contents in miniature, and not a count of what is inside. While
/// focus is on a folded bay a digit, `enter` and the arrows decline and say
/// why, so a mark standing in for those would be offering a press that is
/// refused.
pub(crate) fn folded_head_into(ui: &Ui, pal: &Palette, at: Rect, title: &str) {
    card(ui, pal, at);
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
    // **The word goes after the title and not at the far end**, because a
    // folded row is as wide as the bay was and the two would part company on
    // a wide panel — the mark is one statement, read left to right.
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

/// The bay head, and the whole of what one is.
///
/// A title on the left, the bay's own controls on the right, and a grip at the
/// far right where the mock draws one — laid out into the top [`size::HEAD_H`]
/// of `rect`, with `.bay-head`'s hairline under it.
///
/// Seven call sites on the day it is written, which is [`REGIONS`]'s seven
/// bays.
///
/// The pills and the grip are laid out right to left from the right edge, which
/// is what `justify-content: space-between` on a two-child flex row comes to:
/// the title takes the left and the group takes the right, and the group's own
/// order is its writing order once it is placed.
///
/// The head arrives as a [`Head`] rather than as three arguments, because the
/// four bays that are kinds of their own used to spell theirs out at this call
/// and there is now a second reader of every one of them: [`mcp_pill`] lays the
/// same head out again to find the class capsule in it. One table (`head_of`),
/// one derivation (`head_pills`), and the capsule an operator sees is the
/// capsule a press lands on.
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
    // **Painted where [`head_pills`] puts them rather than laid out again
    // here.** That is [`outputs`]'s arrangement one row down: the capsule that
    // is drawn and the capsule a press lands on are one rectangle, so the
    // Program bay's `solo` cannot come apart from the control
    // [`program_head`] hands to a caller -- nor its class pill from the one
    // [`mcp_pill`] hands over.
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

/// A region's head, painted — the one call all five of the console's headed
/// bays make.
///
/// It was five calls to [`bay_head`] with the title, the pills and the grip
/// written out at each: one copy while the paint was the only reader of them,
/// and four copies the moment [`mcp_pill`] had to lay the same head out again
/// to find a class capsule in it. [`head_of`] is the table now and this is the
/// one caller that turns it into paint.
///
/// A no-op for a region with no head, which is the transport, the Outputs row,
/// a pane, the picture and the preview row.
pub(crate) fn head_into(ui: &Ui, pal: &Palette, rect: Rect, region: &Region, open: Open) {
    if let Some(head) = head_of(region) {
        bay_head(ui, pal, rect, &head, open);
    }
}

/// The box a bay head is painted into: the top [`size::HEAD_H`] of the bay,
/// clipped to the bay itself so that a bay shorter than its own head has a
/// shorter head rather than one drawn over whatever is below it.
///
/// Two readers, which is why it is a function: [`bay_head`] paints into it and
/// [`head_pills`] lays the head's controls out in it. The mid-line a pill is
/// centred on is this box's, so a head clipped short takes its pills up with it
/// and the two cannot disagree about where the row is.
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
