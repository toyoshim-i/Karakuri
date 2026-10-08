use std::path::Path;
use std::sync::Arc;

use karakuri_store::fs::{with_filesystem, MemoryFs};
use karakuri_store::ndjson::Line;
use karakuri_store::record::Record;
use karakuri_store::store::Store;

/// Verifies that starring works in MemoryFs (VFS) without falling back to host std::fs.
#[test]
fn starring_in_memory_fs_works() {
    let mem = Arc::new(MemoryFs::new());
    with_filesystem(mem, || {
        let root = Path::new("/store");
        let store = Store::open(root).unwrap();
        store
            .write_set(
                "wasm_set",
                &[Line::new(Record::Set {
                    id: "wasm_set".into(),
                    v: 1,
                })],
            )
            .unwrap();

        assert!(store.set_favourite("wasm_set", true).unwrap());
        assert_eq!(
            store.favourites().unwrap().into_iter().collect::<Vec<_>>(),
            vec!["wasm_set".to_string()]
        );
        assert!(
            !store.set_favourite("wasm_set", true).unwrap(),
            "already starred"
        );
        assert!(store.set_favourite("wasm_set", false).unwrap(), "unstar");
        assert!(store.favourites().unwrap().is_empty());
    });
}
