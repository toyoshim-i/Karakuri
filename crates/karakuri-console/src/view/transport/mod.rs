use super::*;

pub mod arrangement;
pub mod audio_in;
pub mod look;
pub mod mcp;
pub mod tempo;
pub mod theme;
pub mod tracker;

pub use arrangement::*;
pub(super) use arrangement::{arrangement_into, learn_into, map_into};
pub(super) use audio_in::audio_in_into;
pub use audio_in::*;
pub(super) use look::look_into;
pub(crate) use look::next_tonemap;
pub use look::*;
pub(super) use mcp::mcp_server_into;
pub use mcp::*;
pub(super) use tempo::transport_into;
pub use tempo::*;
pub(super) use theme::theme_into;
pub use theme::*;
pub(super) use tracker::tracker_into;
pub use tracker::*;

impl View {
    /// Declares redraw staleness and budget for the transport row when visible and active (ADR-0193, ADR-0283, P-0094).
    pub(super) fn transport_declares(&self, layout: &karakuri_layout::Layout) -> Option<Declared> {
        let row = layout
            .find("transport")
            .is_some_and(|id| layout.visible(id));
        (row && self.transport.is_some()).then_some(Declared {
            region: "transport",
            cost: PANEL_PASS,
            staleness: BEAT_STALENESS,
            // Staleness and moves_in match as transport advances continuously per frame (ADR-0283, P-0094).
            moves_in: BEAT_STALENESS,
        })
    }
}

/// Computes the rightmost boundary across the transport row and any active pills.
#[allow(dead_code, clippy::too_many_arguments)]
pub fn transport_content_right(
    row: &TransportRow,
    audio: Option<&AudioInPill>,
    tracker: Option<&TrackerGroup>,
    learn: Option<&LearnPill>,
    map: Option<&MapRow>,
    arr: Option<&ArrangementPill>,
    look: Option<&LookRow>,
    mcp: Option<&McpServerPill>,
    theme: Option<&ThemePill>,
) -> f32 {
    let mut right = row.content_right();
    if let Some(p) = audio {
        right = right.max(p.pill.max.x);
    }
    if let Some(g) = tracker {
        right = right.max(g.double.max.x);
        if let Some(ref o) = g.offset {
            right = right.max(o.value.max.x);
        }
    }
    if let Some(l) = learn {
        right = right.max(l.pill.max.x);
    }
    if let Some(m) = map {
        right = right.max(m.pill.max.x);
    }
    if let Some(a) = arr {
        right = right.max(a.pill.max.x);
    }
    if let Some(l) = look {
        right = right.max(l.value.max.x);
    }
    if let Some(m) = mcp {
        right = right.max(m.pill.max.x);
    }
    if let Some(t) = theme {
        right = right.max(t.pill.max.x);
    }
    right
}

/// Computes the maximum scroll offset needed to reveal all transport content.
pub fn transport_max_scroll(content_right: f32, strip: Rect) -> f32 {
    (content_right + size::TRANSPORT_PAD_X - strip.max.x).max(0.0)
}

/// Layout and positioning for all controls, readouts, and pills in the transport row.
#[derive(Debug, Clone, PartialEq)]
pub struct TransportLayout {
    pub max_scroll: f32,
    pub row: TransportRow,
    pub audio: Option<AudioInPill>,
    pub tracker: Option<TrackerGroup>,
    pub learn: Option<LearnPill>,
    pub map: Option<MapRow>,
    pub arrangement: Option<ArrangementPill>,
    pub look: Option<LookRow>,
    pub mcp: Option<McpServerPill>,
    pub theme: Option<ThemePill>,
}

impl View {
    /// Derives the fully configured Transport row layout shifted by active scroll offset.
    pub fn transport_layout(
        &self,
        ctx: &egui::Context,
        layout: &karakuri_layout::Layout,
    ) -> Option<TransportLayout> {
        let row = transport(ctx, layout, self.transport)?;
        let audio = self
            .audio
            .as_ref()
            .and_then(|a| audio_in(ctx, layout, self.transport, Some(a)));
        let tracker = tracker_group(
            ctx,
            layout,
            self.transport,
            self.audio.as_ref(),
            self.tracker,
        );
        let learn = learn_pill(
            ctx,
            layout,
            self.transport,
            self.audio.as_ref(),
            self.tracker,
            self.map.as_ref(),
            self.learn,
        );
        let map = self.map.as_ref().and_then(|m| {
            map_pill(
                ctx,
                layout,
                self.transport,
                self.audio.as_ref(),
                self.tracker,
                Some(m),
            )
        });
        let arr = arrangement(
            ctx,
            layout,
            self.transport,
            self.audio.as_ref(),
            self.tracker,
            self.map.as_ref(),
            &self.arrangement,
        );
        let look_row = look(
            ctx,
            layout,
            self.transport,
            self.audio.as_ref(),
            self.tracker,
            self.map.as_ref(),
            &self.arrangement,
            self.look,
        );
        let mcp = mcp_server_pill(
            ctx,
            layout,
            self.transport,
            self.audio.as_ref(),
            self.tracker,
            self.map.as_ref(),
            &self.arrangement,
            &self.mcp_server,
            self.theme_mode,
            self.theme_menu_open,
        );
        let theme = theme_pill(
            ctx,
            layout,
            self.transport,
            self.audio.as_ref(),
            self.tracker,
            self.map.as_ref(),
            &self.arrangement,
            self.theme_mode,
            self.theme_menu_open,
        );

        let viewport = to_egui(layout.viewport());
        let left_end = [
            look_row.as_ref().map(|l| l.value.max.x),
            arr.as_ref().map(|a| a.pill.max.x),
            map.as_ref().map(|m| m.pill.max.x),
            learn.as_ref().map(|l| l.pill.max.x),
            tracker.as_ref().map(|g| g.double.max.x),
            audio.as_ref().map(|a| a.pill.max.x),
        ]
        .into_iter()
        .flatten()
        .fold(row.bar.max.x, f32::max);

        let mcp_w = mcp.as_ref().map_or(0.0, |m| m.pill.width());
        let theme_w = theme.as_ref().map_or(0.0, |t| t.pill.width());
        let ideal_theme_left = row.frame.min.x - size::TRANSPORT_GAP - theme_w;
        let ideal_mcp_left = if theme.is_some() {
            ideal_theme_left - size::TRANSPORT_GAP - mcp_w
        } else {
            row.frame.min.x - size::TRANSPORT_GAP - mcp_w
        };
        let mcp_left = ideal_mcp_left.max(left_end + size::TRANSPORT_GAP);
        let theme_left = if mcp.is_some() {
            ideal_theme_left.max(mcp_left + mcp_w + size::TRANSPORT_GAP)
        } else {
            ideal_theme_left.max(left_end + size::TRANSPORT_GAP)
        };

        let mcp = mcp.map(|m| {
            let shift = mcp_left - m.pill.min.x;
            m.translated(shift, viewport)
        });
        let theme = theme.map(|t| {
            let shift = theme_left - t.pill.min.x;
            t.translated(shift, viewport)
        });

        let pills_right = [
            theme.as_ref().map(|t| t.pill.max.x),
            mcp.as_ref().map(|m| m.pill.max.x),
            look_row.as_ref().map(|l| l.value.max.x),
            arr.as_ref().map(|a| a.pill.max.x),
            map.as_ref().map(|m| m.pill.max.x),
            learn.as_ref().map(|l| l.pill.max.x),
            tracker.as_ref().map(|g| g.double.max.x),
            audio.as_ref().map(|a| a.pill.max.x),
        ]
        .into_iter()
        .flatten()
        .fold(0.0f32, f32::max);

        let row = row.after_pills(pills_right);
        let content_right = transport_content_right(
            &row,
            audio.as_ref(),
            tracker.as_ref(),
            learn.as_ref(),
            map.as_ref(),
            arr.as_ref(),
            look_row.as_ref(),
            mcp.as_ref(),
            theme.as_ref(),
        );
        let max_scroll = transport_max_scroll(content_right, row.strip);
        let scroll = self.transport_scroll.clamp(0.0, max_scroll);
        let dx = -scroll;

        Some(TransportLayout {
            max_scroll,
            row: row.translated(dx, viewport),
            audio: audio.map(|a| a.translated(dx, viewport)),
            tracker: tracker.map(|g| g.translated(dx, viewport)),
            learn: learn.map(|l| l.translated(dx, viewport)),
            map: map.map(|m| m.translated(dx, viewport)),
            arrangement: arr.map(|a| a.translated(dx, viewport)),
            look: look_row.map(|l| l.translated(dx, viewport)),
            mcp: mcp.map(|m| m.translated(dx, viewport)),
            theme: theme.map(|t| t.translated(dx, viewport)),
        })
    }
}
