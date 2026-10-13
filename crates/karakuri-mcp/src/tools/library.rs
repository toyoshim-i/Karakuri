//! Library and preset exploration tools with multi-dimensional filtering and pagination.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::State;

/// Representation of a library procedure or Set item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LibraryItem {
    pub name: String,
    pub file: String,
    pub layer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topology: Option<String>,
    pub origin: String,
    pub summary: String,
}

/// Resolves the filesystem or virtual path to the presets directory.
pub fn resolve_presets_dir() -> PathBuf {
    if let Ok(Some(p)) = karakuri_environment::places::presets(None) {
        p.dir
    } else if karakuri_store::fs::exists(Path::new("examples")) {
        PathBuf::from("examples")
    } else {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples"))
    }
}

/// Extracts declared topology from shader source text.
pub fn declared_topology(source: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("topology") {
            let top = rest.trim();
            if top.starts_with("points") {
                return Some("points".to_string());
            } else if top.starts_with("lines") {
                return Some("lines".to_string());
            } else if top.starts_with("triangles") {
                return Some("triangles".to_string());
            }
        }
    }
    None
}

/// Extracts the first non-empty comment line as a 1-line description.
pub fn extract_summary(source: &str) -> String {
    for line in source.lines() {
        let trimmed = line.trim();
        let comment = if let Some(c) = trimmed.strip_prefix("///") {
            Some(c)
        } else {
            trimmed.strip_prefix("//")
        };
        if let Some(c) = comment {
            let text = c.trim().trim_matches('*').trim();
            if !text.is_empty() && !text.starts_with("proc ") && !text.starts_with("kind ") {
                return text.to_string();
            }
        }
    }
    String::new()
}

/// Generates a summary description for a `.kset` file.
fn extract_kset_summary(source: &str, stem: &str) -> String {
    let mut comment_summary = String::new();
    let mut parts = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if comment_summary.is_empty() {
            if let Some(c) = trimmed.strip_prefix("//") {
                let text = c.trim().trim_matches('*').trim();
                if !text.is_empty() {
                    comment_summary = text.to_string();
                }
            }
        }
        if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
            if val.get("t").and_then(Value::as_str) == Some("part") {
                if let Some(layer) = val.get("layer").and_then(Value::as_str) {
                    let path = val.get("path").and_then(Value::as_str).unwrap_or("");
                    parts.push(format!("{layer}:{path}"));
                }
            }
        }
    }
    if !comment_summary.is_empty() {
        comment_summary
    } else if !parts.is_empty() {
        format!(
            "Set `{stem}` with {} nodes ({})",
            parts.len(),
            parts.join(", ")
        )
    } else {
        format!("Set `{stem}`")
    }
}

/// Collects all available items across presets (`examples/`) and store (`.karakuri/`).
pub(crate) fn collect_library_items(state: &State) -> Vec<LibraryItem> {
    let mut items = Vec::new();
    let presets_dir = resolve_presets_dir();

    // 1. Presets collection
    if let Ok(entries) = karakuri_store::fs::read_dir(&presets_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let is_file = entry.file_type().map(|ft| !ft.is_dir()).unwrap_or(true);
            if !is_file {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_string();
            let stem = path
                .file_stem()
                .and_then(OsStr::to_str)
                .unwrap_or_default()
                .to_string();

            if file_name.ends_with(".kir") {
                if let Ok(content) = karakuri_store::fs::read_to_string(&path) {
                    let layer = karakuri_environment::history::declared_kind(content.as_bytes())
                        .unwrap_or("L4")
                        .to_string();
                    let topology = declared_topology(&content);
                    let summary = extract_summary(&content);
                    items.push(LibraryItem {
                        name: stem,
                        file: file_name,
                        layer,
                        topology,
                        origin: "preset".to_string(),
                        summary,
                    });
                }
            } else if file_name.ends_with(".kset") {
                if let Ok(content) = karakuri_store::fs::read_to_string(&path) {
                    let summary = extract_kset_summary(&content, &stem);
                    items.push(LibraryItem {
                        name: stem.clone(),
                        file: file_name,
                        layer: "Set".to_string(),
                        topology: None,
                        origin: "preset".to_string(),
                        summary,
                    });
                }
            }
        }
    }

    // 2. Store collection
    if let Ok(store) = karakuri_store::Store::open(&state.store) {
        // Stored sets
        if let Ok(sets) = karakuri_environment::setfile::summarise(&store) {
            for set in sets {
                let node_count = set.nodes.len();
                let summary = format!(
                    "Saved operator Set holding {} node{}",
                    node_count,
                    if node_count == 1 { "" } else { "s" }
                );
                items.push(LibraryItem {
                    name: set.id.clone(),
                    file: format!("{}.kbset", set.id),
                    layer: "Set".to_string(),
                    topology: None,
                    origin: "store".to_string(),
                    summary,
                });
            }
        }

        // Stored standalone procedures
        if let Ok(procs) = store.list_procedures() {
            for proc in procs {
                if let Ok(bytes) = store.read_procedure(&proc.name) {
                    let content = String::from_utf8_lossy(&bytes);
                    let layer = karakuri_environment::history::declared_kind(content.as_bytes())
                        .unwrap_or("L4")
                        .to_string();
                    let topology = declared_topology(&content);
                    let summary = extract_summary(&content);
                    items.push(LibraryItem {
                        name: proc.name.clone(),
                        file: format!("{}.kir", proc.name),
                        layer,
                        topology,
                        origin: "store".to_string(),
                        summary,
                    });
                }
            }
        }
    }

    // Sort stably by layer kind, then by name
    items.sort_by(|a, b| {
        let rank = |layer: &str| match layer {
            "Set" => 0,
            "L1" => 1,
            "L2" => 2,
            "L3" => 3,
            "L4" => 4,
            "Field" => 5,
            "L5" => 6,
            _ => 7,
        };
        rank(&a.layer)
            .cmp(&rank(&b.layer))
            .then_with(|| a.name.cmp(&b.name))
    });

    items
}

/// Executes the `list_library` tool call with multi-dimensional filtering, overview statistics, and pagination.
pub(crate) fn list_library(args: &Value, state: &State) -> Result<String, String> {
    let all_items = collect_library_items(state);

    // Compute library overview statistics
    let total_all = all_items.len();
    let sets_count = all_items.iter().filter(|i| i.layer == "Set").count();
    let l1_count = all_items.iter().filter(|i| i.layer == "L1").count();
    let l2_count = all_items.iter().filter(|i| i.layer == "L2").count();
    let l3_count = all_items.iter().filter(|i| i.layer == "L3").count();
    let l4_count = all_items.iter().filter(|i| i.layer == "L4").count();
    let field_count = all_items.iter().filter(|i| i.layer == "Field").count();
    let l5_count = all_items.iter().filter(|i| i.layer == "L5").count();
    let points_count = all_items
        .iter()
        .filter(|i| i.topology.as_deref() == Some("points"))
        .count();
    let lines_count = all_items
        .iter()
        .filter(|i| i.topology.as_deref() == Some("lines"))
        .count();
    let triangles_count = all_items
        .iter()
        .filter(|i| i.topology.as_deref() == Some("triangles"))
        .count();
    let presets_count = all_items.iter().filter(|i| i.origin == "preset").count();
    let store_count = all_items.iter().filter(|i| i.origin == "store").count();

    // Parse filters
    let scope_filter = args.get("scope").and_then(Value::as_str);
    let layer_filter = args.get("layer").and_then(Value::as_str);
    let topology_filter = args.get("topology").and_then(Value::as_str);
    let query_filter = args.get("query").and_then(Value::as_str);

    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(20)
        .clamp(1, 500) as usize;

    let filtered: Vec<&LibraryItem> = all_items
        .iter()
        .filter(|item| {
            if let Some(scope) = scope_filter {
                match scope.to_ascii_lowercase().as_str() {
                    "presets" | "preset" => {
                        if item.origin != "preset" {
                            return false;
                        }
                    }
                    "store" => {
                        if item.origin != "store" {
                            return false;
                        }
                    }
                    "all" => {}
                    _ => {
                        return false;
                    }
                }
            }
            if let Some(layer) = layer_filter {
                if !item.layer.eq_ignore_ascii_case(layer) {
                    return false;
                }
            }
            if let Some(top) = topology_filter {
                if item
                    .topology
                    .as_deref()
                    .is_none_or(|t| !t.eq_ignore_ascii_case(top))
                {
                    return false;
                }
            }
            if let Some(query) = query_filter {
                let q = query.to_ascii_lowercase();
                let matches_name = item.name.to_ascii_lowercase().contains(&q);
                let matches_summary = item.summary.to_ascii_lowercase().contains(&q);
                if !matches_name && !matches_summary {
                    return false;
                }
            }
            true
        })
        .collect();

    let matched_count = filtered.len();

    let mut out = format!(
        "Library Overview ({total_all} items: {sets_count} Sets, {l1_count} L1, {l2_count} L2, \
         {l3_count} L3, {l4_count} L4, {field_count} Field, {l5_count} L5 | \
         Topology: {points_count} points, {lines_count} lines, {triangles_count} triangles | \
         {presets_count} presets, {store_count} in store)\n\n"
    );

    if matched_count == 0 {
        out.push_str("No items matched the specified filters.\n");
        return Ok(out);
    }

    let paged: Vec<&&LibraryItem> = filtered.iter().skip(offset).take(limit).collect();
    let start = offset + 1;
    let end = offset + paged.len();

    out.push_str(&format!(
        "Showing {start}..{end} of {matched_count} matching items:\n\n"
    ));

    for item in &paged {
        let topology_str = match &item.topology {
            Some(t) => format!(", {t}"),
            None => String::new(),
        };
        let summary_str = if item.summary.is_empty() {
            String::new()
        } else {
            format!(" — {}", item.summary)
        };
        out.push_str(&format!(
            "- `{}` ({}{}) [{}]{}\n",
            item.name, item.layer, topology_str, item.origin, summary_str
        ));
    }

    out.push('\n');
    if end < matched_count {
        out.push_str(&format!(
            "Showing {start}..{end} of {matched_count}. Call `list_library` with `offset: {end}` \
             to view the next page.\n"
        ));
    } else {
        out.push_str(&format!("Showing all {matched_count} matching items.\n"));
    }

    Ok(out)
}
