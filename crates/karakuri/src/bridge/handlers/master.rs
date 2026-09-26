use super::*;

/// Reads the master chain state from the engine for display in the Master bay (ADR-0156).
pub(crate) fn chain_view(
    present: &karakuri_engine::Present,
    offers: &[view::AddChoice],
) -> view::Chain {
    view::Chain {
        slots: present
            .chain_reading()
            .into_iter()
            .map(|slot| view::ChainSlot {
                name: chain_slot_name(&slot.procedure, offers),
                cut: slot.cut.map(mix::cut),
                params: slot
                    .params
                    .into_iter()
                    .map(|p| view::SlotParam {
                        key: p.key,
                        range: [p.min, p.max],
                        value: p.value,
                        default: p.default,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// The word a chain row draws for one address — see [`chain_view`].
fn chain_slot_name(address: &str, offers: &[view::AddChoice]) -> String {
    if let Some(name) = karakuri_environment::mix::shipped::name_of(address) {
        return name.to_owned();
    }
    if let Some(offer) = offers.iter().find(|offer| offer.procedure == address) {
        return offer.words.clone();
    }
    // The short address, which is what a content-addressed thing is called
    // when nothing has named it: the `sha256:` prefix and enough of the digest
    // to tell two apart, as `karakuri_store` prints one.
    address.chars().take(SHORT_ADDRESS).collect()
}

/// How much of a content address a chain row draws where nothing names the
/// procedure: `sha256:` and eight digits.
const SHORT_ADDRESS: usize = 15;

/// Converts an engine `MaskKind` to the vocabulary `WipeKind` (P-0090).
pub(crate) fn wipe_kind(kind: MaskKind) -> karakuri_operation::WipeKind {
    match kind {
        MaskKind::None => karakuri_operation::WipeKind::None,
        MaskKind::Linear => karakuri_operation::WipeKind::Linear,
        MaskKind::Radial => karakuri_operation::WipeKind::Radial,
    }
}

/// Converts a vocabulary `WipeKind` to an engine `MaskKind` (P-0090).
pub(crate) fn mask_kind(kind: karakuri_operation::WipeKind) -> MaskKind {
    match kind {
        karakuri_operation::WipeKind::None => MaskKind::None,
        karakuri_operation::WipeKind::Linear => MaskKind::Linear,
        karakuri_operation::WipeKind::Radial => MaskKind::Radial,
    }
}

/// Returns the display's refresh interval in milliseconds derived from window monitor settings.
pub(crate) fn budget_ms(window: &Window) -> Option<f32> {
    let millihertz = window.current_monitor()?.refresh_rate_millihertz()?;
    match millihertz > 0 {
        true => Some(1.0e6 / millihertz as f32),
        false => None,
    }
}
