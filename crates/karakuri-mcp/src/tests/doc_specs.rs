use std::path::Path;

/// Extract all ```` ```kir ```` code blocks from a markdown string along with their 1-based start line.
fn extract_kir_blocks(markdown: &str) -> Vec<(usize, String)> {
    let mut blocks = Vec::new();
    let mut in_kir = false;
    let mut current_block = String::new();
    let mut start_line = 0;

    for (idx, line) in markdown.lines().enumerate() {
        let trimmed = line.trim();
        if !in_kir {
            if trimmed == "```kir" {
                in_kir = true;
                current_block.clear();
                start_line = idx + 1;
            }
        } else if trimmed == "```" {
            in_kir = false;
            blocks.push((start_line, current_block.clone()));
            current_block.clear();
        } else {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }
    blocks
}

/// Extract environment variable (ambient) names and types from §4 of ir-spec.md.
fn extract_ambient_table(markdown: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    let mut in_table = false;
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## 4. Built-in Environment & Signals") {
            in_table = true;
            continue;
        }
        if in_table && trimmed.starts_with("## ") {
            break;
        }
        if in_table
            && trimmed.starts_with('|')
            && !trimmed.contains("Name")
            && !trimmed.contains("---")
        {
            let cols: Vec<&str> = trimmed.split('|').map(|s| s.trim()).collect();
            if cols.len() >= 3 {
                let name = cols[1].trim_matches('`').trim();
                let ty = cols[2].trim_matches('`').trim();
                if !name.is_empty() && !ty.is_empty() {
                    entries.push((name.to_string(), ty.to_string()));
                }
            }
        }
    }
    entries
}

fn minimal_procedure_for_ambient(name: &str) -> String {
    match name {
        "t" | "beats" | "dt" | "seed" | "source" | "capacity" => format!(
            "proc test_{name} {{\n  kind L1\n  topology points\n  capacity [1, 2] = 1\n  element {{\n    let x = {name};\n  }}\n}}"
        ),
        "copy" => {
            "proc test_copy {\n  kind L2\n  deform {\n    let x = copy;\n  }\n}".to_string()
        }
        "point" => {
            "proc test_point {\n  kind Field\n  field {\n    distance = length(point);\n  }\n}"
                .to_string()
        }
        "camera" => "proc test_camera {\n  kind L4\n  blend additive\n  consumes position\n  vertex {\n    clip = camera * vec4(position, 1.0);\n    point_rate = 1.0;\n  }\n  fragment {\n    color = vec4(1.0);\n  }\n}".to_string(),
        "point_coord" => "proc test_point_coord {\n  kind L4\n  blend additive\n  fragment {\n    color = vec4(point_coord, 0.0, 1.0);\n  }\n}".to_string(),
        "eye" => "proc test_eye {\n  kind L4\n  blend additive\n  fragment {\n    color = vec4(eye, 1.0);\n  }\n}".to_string(),
        "ray" => "proc test_ray {\n  kind L4\n  blend additive\n  fragment {\n    color = vec4(ray, 1.0);\n  }\n}".to_string(),
        other => panic!("unknown ambient `{other}` in documentation table"),
    }
}

fn check_doc_file(relative_path: &str) {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root");
    let file_path = workspace_root.join(relative_path);
    let content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", file_path.display()));

    let blocks = extract_kir_blocks(&content);
    assert!(
        !blocks.is_empty(),
        "no ```kir blocks found in {}",
        file_path.display()
    );

    for (start_line, code) in blocks {
        let report = crate::check_procedure(&code);
        assert!(
            report.success,
            "Failed checking kir block in {} at line {}:\n---\n{}\n---\nDiagnostics:\n{:#?}",
            relative_path, start_line, code, report.diagnostics
        );
    }
}

#[test]
fn ir_spec_code_blocks_are_valid_procedures() {
    check_doc_file("docs/ir-spec.md");
}

#[test]
fn vr_projection_code_blocks_are_valid_procedures() {
    check_doc_file("docs/vr-projection.md");
}

#[test]
fn ir_spec_environment_table_entries_are_valid_ambients() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root");
    let file_path = workspace_root.join("docs/ir-spec.md");
    let content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", file_path.display()));

    let ambients = extract_ambient_table(&content);
    assert!(
        !ambients.is_empty(),
        "no ambient table entries found in docs/ir-spec.md"
    );

    for (name, ty) in ambients {
        let ambient = karakuri_ir::ast::Ambient::from_name(&name)
            .unwrap_or_else(|| panic!("table entry `{name}` is not a valid Ambient"));
        assert_eq!(
            ambient.ty().name(),
            ty,
            "type mismatch for ambient `{name}`"
        );

        let proc_src = minimal_procedure_for_ambient(&name);
        let report = crate::check_procedure(&proc_src);
        assert!(
            report.success,
            "Minimal procedure for ambient `{name}` failed check:\n---\n{proc_src}\n---\nDiagnostics:\n{:#?}",
            report.diagnostics
        );
    }
}
