use super::*;

/// The master chain, as the console reads it — a level's reading rather than
/// the level (ADR-0156), and the one place the engine's chain becomes the
/// panel's.
///
/// Not `mix::current_chain`, and the two are not the same reading. That one
/// answers the *conversion* — the slot list `written` completes a record from —
/// and this one answers what the bay draws: one row per slot, in the chain's
/// order, each with the name it is drawn under, the cut it reads where its
/// procedure declares `retains`, and every declared parameter with the range it
/// was declared over.
///
/// It reads the *built* chain and not the description beside it: a declared
/// range and a declared name are on the compiled procedure and nowhere else, and
/// `Present::chain_reading` is where they are.
///
/// What a row is named: the three procedures this repository ships are named by
/// the words the bay draws for them; anything else is named by whatever the
/// library lists that address under, and by the short address where nothing
/// does.
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

/// The engine's mask shape, as the vocabulary's — [`blend_mode`]'s function one
/// control along, and the one place these two lists are made to agree.
///
/// A match, so the day a fourth `MaskKind` lands in the engine this stops
/// compiling rather than reading a shape the vocabulary cannot name into a
/// record that has to name one. The mirror image of it is `view::Mixer::mask`,
/// which turns the console's own word into the same vocabulary — three names
/// for three shapes, which is the cost `karakuri-operation` pays for depending
/// on nothing (P-0090).
pub(crate) fn wipe_kind(kind: MaskKind) -> karakuri_operation::WipeKind {
    match kind {
        MaskKind::None => karakuri_operation::WipeKind::None,
        MaskKind::Linear => karakuri_operation::WipeKind::Linear,
        MaskKind::Radial => karakuri_operation::WipeKind::Radial,
    }
}

/// The vocabulary's mask shape, as the engine's — [`wipe_kind`] read the other
/// way, and the two are a pair rather than one function because the crossing
/// happens in both directions in this file.
///
/// The console holds the shape the *next* wipe takes as a
/// [`karakuri_operation::WipeKind`] — `TransitionSetting::WipeShape` is what
/// its pill emits — and `mix::current_transition` takes the engine's, so a
/// wipe's front crosses here on its way to the reading. It crosses back inside
/// that function, through `mix`'s own `wipe_kind`, which is the one place
/// `karakuri-environment` makes the two lists agree: what a record carries is
/// the vocabulary's word either way, and this round trip is the price of a
/// signature that speaks the engine's types to a caller holding them
/// (`karakuri-cli` is that caller).
///
/// A match for [`wipe_kind`]'s reason, so a fourth shape on either side stops
/// the build here rather than at a record naming a shape nothing can read.
pub(crate) fn mask_kind(kind: karakuri_operation::WipeKind) -> MaskKind {
    match kind {
        karakuri_operation::WipeKind::None => MaskKind::None,
        karakuri_operation::WipeKind::Linear => MaskKind::Linear,
        karakuri_operation::WipeKind::Radial => MaskKind::Radial,
    }
}

/// What a frame has to fit in on this window: the display's refresh interval,
/// in milliseconds — the `/16.6` in the mock's transport, at the 60 Hz it was
/// drawn against.
///
/// It is the refresh interval because that is what this window is held to. The
/// surface is `PresentMode::Fifo`, so a frame that takes longer than one
/// interval to build is a frame that misses a vsync, and every millisecond
/// under it is the headroom the mock's own tooltip is about. `Cost::wait` is
/// the other side of the same number: at 60 Hz most of the frame is spent
/// blocked in `get_current_texture` waiting for it.
///
/// It is not `karakuri_engine`'s `DEFAULT_BUDGET_MS`, which is 20 and is a
/// different budget with the same word on it: that one is what a *candidate
/// Set* has to hold to survive a hot swap, measured offscreen at a fixed size
/// and judged on a median. The mock's tooltip runs the two together — *"12.4 of
/// 16.6 — there is headroom. A candidate that cannot hold this is rolled back
/// on its own"* — and they are two numbers. This row draws the one the frame is
/// actually against.
///
/// `None` where `winit` will not say, which is a monitor it cannot name or a
/// mode with no refresh rate on it. The row then draws the frame time and no
/// budget, rather than a plausible 16.6 nothing measured.
///
/// Read once, when the window opens. A window dragged onto a 120 Hz display
/// keeps the interval it opened on, which is a real limitation and is the price
/// of not asking the platform for a monitor handle sixty times a second.
pub(crate) fn budget_ms(window: &Window) -> Option<f32> {
    let millihertz = window.current_monitor()?.refresh_rate_millihertz()?;
    match millihertz > 0 {
        true => Some(1.0e6 / millihertz as f32),
        false => None,
    }
}
