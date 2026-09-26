use super::*;

/// Drains engine swap events to update Staging lane candidate rows and transport health (ADR-0164, ADR-0310, ADR-0316, ADR-0326).
///
/// Returns `true` if a live Set was swapped into place this frame.
pub(crate) fn staging(
    deck: &mut Deck,
    // Keeping tracks slot material and per-node compilation hashes to compute staging diffs (ADR-0326).
    keeping: &mut Keeping,
    aims: &[Aiming],
    out: &mut Vec<view::Candidate>,
    // Transport health capsule reflects the most recent build outcome across frames.
    health: &mut Option<view::Stage>,
) -> bool {
    let mut landed = false;
    for slot in 0..deck.slot_count() {
        let addr = EngineSlot(slot as u8);
        // Drain events synchronously before accessing slot state.
        let events: Vec<Event> = deck.events(addr).collect();
        // What the build that landed changed, kept across the drain.
        let mut diffed: Vec<(u64, Vec<Changed>)> = Vec::new();
        for event in events {
            // Forward swap and compilation events to MCP client if active (P-0083).
            if let Some(mcp) = keeping.mcp.as_ref() {
                mcp.swap(slot, &event.to_string());
            }
            if let Event::Swapped { id, .. } = &event {
                let changed = keeping.took_up(aims, slot, *id);
                diffed.push((*id, changed_rows(deck.slot(addr).set(), &changed)));
            }
            // Record whether a newly installed Set actually landed on air (ADR-0316).
            landed |= matches!(verdict(&event), Verdict::Waiting(_, view::Stage::Landed));
            let said: &[String] = match &event {
                Event::SourceRefused { said, .. } => said,
                _ => &[],
            };
            // Diff against prior build nodes or fall back to slot-level row (ADR-0326).
            let changed: &[Changed] = match &event {
                Event::Swapped { id, .. } | Event::Overloaded { id, .. } => diffed
                    .iter()
                    .find(|(built, _)| built == id)
                    .map_or(&[][..], |(_, rows)| rows.as_slice()),
                _ => &[],
            };
            match verdict(&event) {
                Verdict::Waiting(label, stage) => {
                    // Update transport health with the latest verdict and update lane rows.
                    *health = Some(stage);
                    settle(out, slot, label, stage, said, changed)
                }
                Verdict::Settled => out.retain(|row| row.deck != slot),
                Verdict::Nothing => {}
            }
        }
    }
    landed
}

/// A changed node represented as a lane row, containing its operation address,
/// display address, and node name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Changed {
    pub(crate) at: karakuri_operation::NodeAddress,
    pub(crate) addr: String,
    pub(crate) name: String,
}

/// Converts changed node addresses reported by the watcher into lane rows resolved
/// against the installed `Set`. Unnamed nodes are skipped.
pub(crate) fn changed_rows(set: &Set, changed: &[(&'static str, u32)]) -> Vec<Changed> {
    if changed.is_empty() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for name in set.node_names() {
        let Some((layer, index)) = set.node_named(name) else {
            continue;
        };
        if !changed
            .iter()
            .any(|(at, of)| *at == setfile::kind_name(layer) && *of == index)
        {
            continue;
        }
        rows.push(Changed {
            at: karakuri_operation::NodeAddress {
                layer: asked_layer(layer),
                index,
            },
            addr: node_addr(layer, index),
            name: name.clone(),
        });
    }
    rows
}

/// Lane state transition triggered by a swap [`Event`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict<'a> {
    /// The slot has a row, under this name and on this word.
    Waiting(&'a str, view::Stage),
    /// The slot has no row: its file and its picture agree.
    Settled,
    /// The lane does not change.
    Nothing,
}

/// See [`Verdict`] and [`staging`], which is where each arm is argued.
pub(crate) fn verdict(event: &Event) -> Verdict<'_> {
    match event {
        Event::Swapped { label, .. } => Verdict::Waiting(label, view::Stage::Landed),
        Event::Rejected { label, .. } => Verdict::Waiting(label, view::Stage::Refused),
        Event::Overloaded { label, .. } => Verdict::Waiting(label, view::Stage::Overloaded),
        // Source refused by the checker; file edits differ from playing state.
        Event::SourceRefused { label, .. } => Verdict::Waiting(label, view::Stage::NotCompiled),
        Event::Accepted { .. } => Verdict::Settled,
        Event::WorkerLost => Verdict::Nothing,
    }
}

/// Replaces a slot's rows in-place in deck and node order (ADR-0326).
///
/// Rows reflect the latest build's changed nodes, matching the Inspector's order.
pub(crate) fn settle(
    out: &mut Vec<view::Candidate>,
    deck: usize,
    label: &str,
    stage: view::Stage,
    // Checker diagnostic messages, populated only on refusal.
    said: &[String],
    // Nodes changed in this build. If empty, a single unaddressed row is displayed.
    changed: &[Changed],
) {
    let at = out.partition_point(|row| row.deck < deck);
    let end = at + out[at..].partition_point(|row| row.deck == deck);
    let rows: Vec<view::Candidate> = match changed.is_empty() {
        true => vec![view::Candidate {
            deck,
            at: None,
            addr: String::new(),
            name: label.to_owned(),
            stage,
            said: said.to_vec(),
        }],
        false => changed
            .iter()
            .map(|node| view::Candidate {
                deck,
                at: Some(node.at),
                addr: node.addr.clone(),
                name: node.name.clone(),
                stage,
                said: said.to_vec(),
            })
            .collect(),
    };
    out.splice(at..end, rows);
}
