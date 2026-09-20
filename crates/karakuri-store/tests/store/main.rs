//! Integration tests for `Store` and the ndjson I/O, run against a
//! temporary directory — never the user's `library/`.

use std::fs;
use std::time::{Duration, SystemTime};
use tempfile::tempdir;

use karakuri_store::{project, Hash, Layer, Line, NodeAddress, Record, Store, StoreError, Value};

mod arrangements;
mod artifacts;
mod procedures;
mod sets;

pub use sets::a_set;
