use super::*;
use std::collections::HashSet;
use std::path::Path;

#[test]
fn list_library_discovers_all_examples_from_examples_suite() {
    let (_tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(state(rx)));

    let req = json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": {
                "scope": "presets",
                "limit": 500
            }
        }
    });

    let rep = dispatch(&req, &state)
        .settled()
        .expect("list_library reply");
    assert_ne!(
        rep["result"]["isError"], true,
        "list_library failed: {rep:?}"
    );
    let text = rep["result"]["content"][0]["text"].as_str().expect("text");
    assert!(
        text.contains("Library Overview"),
        "overview missing in {text}"
    );

    // Inspect files in examples/
    let examples_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut expected_files = HashSet::new();
    for entry in std::fs::read_dir(&examples_dir).expect("read examples/") {
        let entry = entry.expect("entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".kir") || name.ends_with(".kset") {
            let stem = entry
                .path()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            expected_files.insert(stem);
        }
    }

    assert!(
        expected_files.len() >= 250,
        "expected at least 250 example files, found {}",
        expected_files.len()
    );

    // Verify all files from examples suite appear in the output
    for stem in expected_files {
        assert!(
            text.contains(&format!("`{stem}`")),
            "example `{stem}` missing from list_library output"
        );
    }
}

#[test]
fn list_library_multi_dimensional_filtering() {
    let (_tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(state(rx)));

    // 1. Filter by Layer L1
    let req = json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": {
                "layer": "L1",
                "limit": 200
            }
        }
    });
    let rep = dispatch(&req, &state).settled().expect("rep");
    let text = rep["result"]["content"][0]["text"].as_str().expect("text");
    assert!(
        text.contains("(L1"),
        "output should contain L1 items: {text}"
    );
    assert!(
        !text.contains("(L4"),
        "output should not contain L4 items: {text}"
    );
    assert!(
        !text.contains("(Set"),
        "output should not contain Set items: {text}"
    );

    // 2. Filter by Topology lines
    let req_top = json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": {
                "topology": "lines",
                "limit": 100
            }
        }
    });
    let rep_top = dispatch(&req_top, &state).settled().expect("rep");
    let text_top = rep_top["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        text_top.contains(", lines)"),
        "output should contain lines items: {text_top}"
    );
    assert!(
        !text_top.contains(", points)"),
        "output should not contain points items: {text_top}"
    );

    // 3. Search query
    let req_query = json!({
        "jsonrpc": "2.0", "id": 4, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": {
                "query": "bloom",
                "limit": 50
            }
        }
    });
    let rep_query = dispatch(&req_query, &state).settled().expect("rep");
    let text_query = rep_query["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        text_query.contains("`bloom`"),
        "query for bloom should find bloom: {text_query}"
    );
}

#[test]
fn list_library_deterministic_pagination() {
    let (_tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(state(rx)));

    let req_p1 = json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": { "offset": 0, "limit": 10 }
        }
    });
    let rep_p1 = dispatch(&req_p1, &state).settled().expect("rep");
    let text_p1 = rep_p1["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(text_p1.contains("Showing 1..10"), "p1 header: {text_p1}");

    let req_p2 = json!({
        "jsonrpc": "2.0", "id": 6, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": { "offset": 10, "limit": 10 }
        }
    });
    let rep_p2 = dispatch(&req_p2, &state).settled().expect("rep");
    let text_p2 = rep_p2["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(text_p2.contains("Showing 11..20"), "p2 header: {text_p2}");
}

#[test]
fn read_procedure_by_name_reads_unmounted_preset() {
    let (_tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(state(rx)));

    let req = json!({
        "jsonrpc": "2.0", "id": 7, "method": "tools/call",
        "params": {
            "name": "read_procedure",
            "arguments": { "name": "bloom" }
        }
    });
    let rep = dispatch(&req, &state).settled().expect("rep");
    assert_ne!(
        rep["result"]["isError"], true,
        "reading bloom failed: {rep:?}"
    );
    let source = rep["result"]["content"][0]["text"].as_str().expect("text");
    assert!(
        source.contains("proc bloom"),
        "source should contain proc bloom: {source}"
    );
    assert!(
        source.contains("kind L5"),
        "source should declare kind L5: {source}"
    );

    // Reading non-existent procedure should fail gracefully
    let req_missing = json!({
        "jsonrpc": "2.0", "id": 8, "method": "tools/call",
        "params": {
            "name": "read_procedure",
            "arguments": { "name": "non_existent_procedure_xyz" }
        }
    });
    let rep_missing = dispatch(&req_missing, &state).settled().expect("rep");
    assert_eq!(rep_missing["result"]["isError"], true);
}

#[test]
fn read_set_reads_preset_and_includes_constituent_sources() {
    let (_tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(state(rx)));

    let req = json!({
        "jsonrpc": "2.0", "id": 9, "method": "tools/call",
        "params": {
            "name": "read_set",
            "arguments": { "id": "beat_cloud" }
        }
    });
    let rep = dispatch(&req, &state).settled().expect("rep");
    assert_ne!(rep["result"]["isError"], true, "read_set failed: {rep:?}");
    let text = rep["result"]["content"][0]["text"].as_str().expect("text");
    assert!(
        text.contains("preset set `beat_cloud`"),
        "should name preset: {text}"
    );
    assert!(
        text.contains("```kir"),
        "should inline procedure code: {text}"
    );
    assert!(
        text.contains("proc beat_shell") && text.contains("proc soft_points"),
        "should contain shader code: {text}"
    );
}

#[test]
fn library_documentation_topic_is_readable() {
    let (_tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(state(rx)));

    let req = json!({
        "jsonrpc": "2.0", "id": 10, "method": "tools/call",
        "params": {
            "name": "doc_read",
            "arguments": { "id": "library" }
        }
    });
    let rep = dispatch(&req, &state).settled().expect("rep");
    assert_ne!(rep["result"]["isError"], true);
    let text = rep["result"]["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("Library & Preset Exploration"), "{text}");
}

#[test]
fn library_gate_enforces_permissions_and_privacy() {
    let (_tx, rx) = mpsc::channel();
    let s = state(rx);
    let opening = s.opening.clone();
    let state = Arc::new(Mutex::new(s));

    // 1. Default is Preset
    let req_perm = json!({
        "jsonrpc": "2.0", "id": 11, "method": "tools/call",
        "params": { "name": "get_permissions", "arguments": {} }
    });
    let rep_perm = dispatch(&req_perm, &state).settled().expect("rep");
    let text_perm = rep_perm["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        text_perm.contains("\"library\": \"preset\""),
        "should default to preset: {text_perm}"
    );
    assert!(
        text_perm.contains("\"policy\": \"on\""),
        "slots should default to on: {text_perm}"
    );

    // Preset: list_library with scope: "store" is refused
    let req_store = json!({
        "jsonrpc": "2.0", "id": 12, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": { "scope": "store" }
        }
    });
    let rep_store = dispatch(&req_store, &state).settled().expect("rep");
    assert_eq!(rep_store["result"]["isError"], true);
    let err_text = rep_store["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        err_text.contains("not permitted while the Library gate is preset"),
        "{err_text}"
    );

    // Preset: list_library without scope has store withheld notice
    let req_all = json!({
        "jsonrpc": "2.0", "id": 13, "method": "tools/call",
        "params": { "name": "list_library", "arguments": {} }
    });
    let rep_all = dispatch(&req_all, &state).settled().expect("rep");
    let text_all = rep_all["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        text_all.contains("store withheld: Library gate is preset"),
        "{text_all}"
    );

    // 2. Off gate: everything is refused
    opening.set(
        karakuri_operation::gate::Open::CLOSED
            .with_library(karakuri_operation::gate::LibraryGate::Off),
    );
    let req_off_list = json!({
        "jsonrpc": "2.0", "id": 14, "method": "tools/call",
        "params": { "name": "list_library", "arguments": {} }
    });
    let rep_off_list = dispatch(&req_off_list, &state).settled().expect("rep");
    assert_eq!(rep_off_list["result"]["isError"], true);

    let req_off_proc = json!({
        "jsonrpc": "2.0", "id": 15, "method": "tools/call",
        "params": { "name": "read_procedure", "arguments": { "name": "bloom" } }
    });
    let rep_off_proc = dispatch(&req_off_proc, &state).settled().expect("rep");
    assert_eq!(rep_off_proc["result"]["isError"], true);

    let req_off_set = json!({
        "jsonrpc": "2.0", "id": 16, "method": "tools/call",
        "params": { "name": "read_set", "arguments": { "id": "beat_cloud" } }
    });
    let rep_off_set = dispatch(&req_off_set, &state).settled().expect("rep");
    assert_eq!(rep_off_set["result"]["isError"], true);

    // 3. All gate: scope: "store" is permitted
    opening.set(
        karakuri_operation::gate::Open::CLOSED
            .with_library(karakuri_operation::gate::LibraryGate::All),
    );
    let req_all_store = json!({
        "jsonrpc": "2.0", "id": 17, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": { "scope": "store" }
        }
    });
    let rep_all_store = dispatch(&req_all_store, &state).settled().expect("rep");
    assert_ne!(
        rep_all_store["result"]["isError"], true,
        "{rep_all_store:?}"
    );
}

#[test]
fn list_library_items_expose_content_hashes() {
    let (_tx, rx) = mpsc::channel();
    let state = Arc::new(Mutex::new(state(rx)));

    let req = json!({
        "jsonrpc": "2.0", "id": 18, "method": "tools/call",
        "params": {
            "name": "list_library",
            "arguments": {
                "layer": "L5",
                "limit": 10
            }
        }
    });

    let rep = dispatch(&req, &state).settled().expect("rep");
    assert_ne!(rep["result"]["isError"], true, "{rep:?}");
    let text = rep["result"]["content"][0]["text"].as_str().expect("text");
    assert!(
        text.contains("`sha256:"),
        "list_library output should expose content hashes: {text}"
    );
    assert!(
        text.contains("`bloom` (L5) [preset] `sha256:"),
        "bloom should include its sha256 hash: {text}"
    );
}
