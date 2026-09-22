use super::*;

/// The word in the `rec` pill, and the mock's `&#9679; rec` without its mark:
/// the mark is drawn rather than typed, which is [`Mask`]'s rule for the same
/// reason — *"whether `◯` and `◑` are in `egui`'s default face is a question
/// with no good answer, and a circle is the same mark either way"*.
pub(super) const REC_LABEL: &str = "rec";

/// The faint word beside the number, and the mock's own capitalisation this
/// time: `.transport`'s `BPM` is upper-case in the markup rather than in CSS,
/// so it is upper-case here.
pub(super) const BPM_LABEL: &str = "BPM";

/// The arithmetic of the row, away from the type it measures and the layout it
/// reads.
#[allow(clippy::too_many_arguments)]
pub(super) fn transport_row(
    row: Rect,
    t: &Transport,
    bpm_w: f32,
    label_w: f32,
    bar_w: f32,
    frame_w: f32,
    health_w: Option<f32>,
    rec_w: Option<f32>,
) -> Option<TransportRow> {
    let mid = row.center().y;
    let span_h = size::BASE * size::LINE;
    let bpm = Rect::from_min_size(
        Pos2::new(row.min.x + size::TRANSPORT_PAD_X, mid - size::BPM_H * 0.5),
        egui::vec2(bpm_w, size::BPM_H),
    );
    let label = Rect::from_min_size(
        Pos2::new(bpm.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(label_w, span_h),
    );
    let dots = t.dots();
    let grid = Rect::from_min_size(
        Pos2::new(label.max.x + size::TRANSPORT_GAP, mid - size::BEAT_H * 0.5),
        egui::vec2(
            size::BEAT_W * dots as f32 + size::BEAT_GAP * (dots - 1) as f32,
            size::BEAT_H,
        ),
    );
    let bar = Rect::from_min_size(
        Pos2::new(grid.max.x + size::TRANSPORT_GAP, mid - span_h * 0.5),
        egui::vec2(bar_w, span_h),
    );
    let right = row.max.x - size::TRANSPORT_PAD_X;
    let rec = rec_w.map(|w| {
        Rect::from_min_size(
            Pos2::new(right - w, mid - size::PILL_H * 0.5),
            egui::vec2(w, size::PILL_H),
        )
    });
    let health_end = match rec {
        Some(pill) => pill.min.x - size::TRANSPORT_GAP,
        None => right,
    };
    let health = health_w.map(|w| {
        Rect::from_min_size(
            Pos2::new(health_end - w, mid - size::PILL_H * 0.5),
            egui::vec2(w, size::PILL_H),
        )
    });
    let frame_end = match health {
        Some(pill) => pill.min.x - size::TRANSPORT_GAP,
        None => health_end,
    };
    let frame = Rect::from_min_size(
        Pos2::new(frame_end - frame_w, mid - span_h * 0.5),
        egui::vec2(frame_w, span_h),
    );

    match row.contains_rect(bpm)
        && row.contains_rect(frame)
        && health.is_none_or(|pill| row.contains_rect(pill))
        && rec.is_none_or(|pill| row.contains_rect(pill))
        && frame.min.x >= bar.max.x + size::TRANSPORT_GAP
    {
        true => Some(TransportRow {
            bpm,
            label,
            grid,
            dots,
            at: t.position(),
            bar,
            frame,
            health,
            rec,
            values: *t,
        }),
        false => None,
    }
}

/// Layout job for the tempo value, formatting to one decimal place and applying
/// a horizontal color gradient across glyphs.
pub(super) fn bpm_job(t: &Transport, from: Color32, to: Color32) -> LayoutJob {
    let text = bpm_text(t);
    let mut job = LayoutJob::default();
    let last = text.chars().count().saturating_sub(1).max(1) as f32;
    for (n, (at, ch)) in text.char_indices().enumerate() {
        job.append(
            &text[at..at + ch.len_utf8()],
            0.0,
            TextFormat {
                font_id: FontId::new(size::BPM_SIZE, FontFamily::Proportional),
                extra_letter_spacing: size::BPM_TRACKING,
                color: mix(from, to, n as f32 / last),
                ..Default::default()
            },
        );
    }
    job
}

/// The tempo, as the mock writes it.
pub(super) fn bpm_text(t: &Transport) -> String {
    format!("{:.1}", t.bpm)
}

/// The bar, as the mock writes it: `bar 37`.
pub(super) fn bar_text(t: &Transport) -> String {
    format!("bar {}", t.bar())
}

/// Layout job for the frame readout (`fps · ms/budget`).
pub(super) fn frame_job(t: &Transport, val: Color32, faint: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let mut push = |text: String, colour: Color32| {
        job.append(
            &text,
            0.0,
            TextFormat {
                font_id: FontId::new(size::BASE, FontFamily::Proportional),
                color: colour,
                ..Default::default()
            },
        );
    };
    if let Some(fps) = t.fps {
        push(format!("{fps:.0}"), val);
        push(" fps · ".to_owned(), faint);
    }
    push("cpu ".to_owned(), faint);
    push(format!("{:.1}", t.frame_ms), val);
    match t.budget_ms {
        Some(budget) => push(format!("/{budget:.1} ms"), faint),
        None => push(" ms".to_owned(), faint),
    }
    if let Some(chain) = t.chain_ms.filter(|ms| *ms > 0.0) {
        push(" + ".to_owned(), faint);
        push(format!("{chain:.2}"), val);
        push(" chain".to_owned(), faint);
    }
    job
}

/// A plain span of the console's own type: `font-size: 11px`, one colour.
pub(super) fn span(text: &str, colour: Color32) -> LayoutJob {
    LayoutJob::simple_singleline(
        text.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        colour,
    )
}

/// Two colours mixed, `t` of the way from the first to the second.
pub(super) fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let at = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Color32::from_rgb(at(a.r(), b.r()), at(a.g(), b.g()), at(a.b(), b.b()))
}
