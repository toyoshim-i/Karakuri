//! The three procedures this repository ships as the master chain's presets,
//! and their content addresses.
//!
//! They were `master.wgsl`'s three fragment entry points until 2026-09-10 and
//! are `.kir` files now (ADR-0340). They are compiled in rather than read from
//! disk for one reason: an address has to be the same number on every machine
//! and in every working directory, and a file read relative to a cwd is not
//! that. Putting them in a store is a separate act, done by whoever is
//! recording — `store.put_artifact(source)` — exactly as a Set's sources are,
//! so a run that records nothing creates nothing.

use std::sync::OnceLock;

/// `examples/feedback.kir` — the one of the three that declares `retains`.
pub const FEEDBACK: &str = include_str!("../../../../examples/feedback.kir");
/// `examples/bloom.kir` — one 9x9 pass where the hand-written form was two.
pub const BLOOM: &str = include_str!("../../../../examples/bloom.kir");
/// `examples/rgb_shift.kir`.
pub const RGB_SHIFT: &str = include_str!("../../../../examples/rgb_shift.kir");

/// The three, in the order the Master bay draws them.
pub const ALL: [(&str, &str); 3] = [
    ("feedback", FEEDBACK),
    ("bloom", BLOOM),
    ("rgb_shift", RGB_SHIFT),
];

/// The content address of one shipped source, spelled the way a record spells
/// one.
pub fn address(source: &str) -> String {
    // `Display` already writes the `sha256:` prefix — see
    // `karakuri_store::hash::Hash`, whose `FromStr` requires it.
    karakuri_store::hash::Hash::of(source.as_bytes()).to_string()
}

/// The three addresses, computed once. Hashing three files is a few
/// microseconds and it is still done once, because this is asked per press.
pub fn addresses() -> &'static karakuri_operation_record::Shipped {
    static ONCE: OnceLock<karakuri_operation_record::Shipped> = OnceLock::new();
    ONCE.get_or_init(|| karakuri_operation_record::Shipped {
        feedback: address(FEEDBACK),
        bloom: address(BLOOM),
        rgb_shift: address(RGB_SHIFT),
    })
}

/// The source one address names, where it is one of the three.
///
/// This is the only resolver that needs no store, which is what lets a windowed
/// run with no store at all put the shipped presets in its chain. Anything else
/// is the store's to answer, and a stream naming an address nothing holds is
/// refused with the address in the message.
pub fn source(address_of: &str) -> Option<&'static str> {
    ALL.into_iter()
        .map(|(_, src)| src)
        .find(|src| address(src) == address_of)
}
