use egui::{CornerRadius, Stroke, StrokeKind};

use super::super::audio_in::Rec;
use super::layout::{bar_text, bpm_job, frame_job, mix, span, BPM_LABEL, REC_LABEL};
use super::*;

/// The transport row's contents: the tempo, the beat grid, the bar and the
/// frame readout.
pub(crate) fn transport_into(ui: &Ui, pal: &Palette, row: &TransportRow) {
    let t = &row.values;
    let painter = ui.painter();
    let centred = |rect: Rect, galley: std::sync::Arc<egui::Galley>| {
        painter.galley(
            Pos2::new(rect.min.x, rect.center().y - galley.size().y * 0.5),
            galley,
            pal.text,
        );
    };

    centred(row.bpm, painter.layout_job(bpm_job(t, pal.mint, pal.lav)));
    centred(row.label, painter.layout_job(span(BPM_LABEL, pal.faint)));

    let radius = CornerRadius::same((size::BEAT_H * 0.5) as u8);
    for index in 0..row.dots {
        let dot = row.dot(index);
        let lit = row.lit(index);
        if lit > 0.0 {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: size::BEAT_GLOW,
                    spread: 0,
                    color: pal.glow_pink.gamma_multiply(lit),
                }
                .as_shape(dot, radius),
            );
        }
        painter.rect_filled(dot, radius, mix(pal.line, pal.pink, lit));
    }

    centred(row.bar, painter.layout_job(span(&bar_text(t), pal.dim)));
    centred(
        row.frame,
        painter.layout_job(frame_job(t, pal.text, pal.faint)),
    );

    if let (Some(rect), Some(stage)) = (row.health, t.health) {
        pill_into(ui, pal, rect, stage.word(), stage == Stage::Landed);
    }

    if let (Some(rect), Some(rec)) = (row.rec, t.rec) {
        rec_into(ui, pal, rect, rec);
    }
}

/// The `● rec` pill: a capsule, a round mark, and the word after it.
fn rec_into(ui: &Ui, pal: &Palette, rect: Rect, rec: Rec) {
    let painter = ui.painter();
    let radius = CornerRadius::same((rect.height() * 0.5) as u8);
    let ink = match rec {
        Rec::Running => {
            painter.add(
                egui::epaint::Shadow {
                    offset: [0, 0],
                    blur: ON_GLOW,
                    spread: 0,
                    color: pal.glow_pink,
                }
                .as_shape(rect, radius),
            );
            painter.rect_filled(rect, radius, tint(pal.pink, ON_WASH));
            pal.pink
        }
        Rec::Idle => {
            painter.rect_stroke(rect, radius, Stroke::new(1.0, pal.line), StrokeKind::Inside);
            pal.dim
        }
    };
    let dot = Rect::from_min_size(
        Pos2::new(
            rect.min.x + size::PILL_PAD_X,
            rect.center().y - size::SINK_DOT * 0.5,
        ),
        egui::vec2(size::SINK_DOT, size::SINK_DOT),
    );
    painter.circle_filled(dot.center(), size::SINK_DOT * 0.5, ink);
    let galley = painter.layout_no_wrap(
        REC_LABEL.to_owned(),
        FontId::new(size::BASE, FontFamily::Proportional),
        ink,
    );
    painter.galley(
        Pos2::new(
            dot.max.x + size::SINK_GAP,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        ink,
    );
}
