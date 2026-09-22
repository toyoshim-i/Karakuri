use super::*;
use crate::hash::Hash;

mod session;
mod set;

fn round_trip(line: &str) -> Record {
    let rec: Record = serde_json::from_str(line).expect("parse");
    let back = serde_json::to_string(&rec).expect("serialise");
    let again: Record = serde_json::from_str(&back).expect("reparse");
    assert_eq!(rec, again);
    rec
}

/// The same, and the bytes have to match too.
fn round_trip_verbatim(line: &str) -> Record {
    let rec = round_trip(line);
    assert_eq!(
        serde_json::to_string(&rec).expect("serialise"),
        line,
        "the record did not come back as the bytes it went in as"
    );
    rec
}
