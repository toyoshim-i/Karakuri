use std::path::Path;

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

#[test]
fn documentation_kir_code_blocks_are_valid_procedures() {
    let engine = crate::resources::doc_engine();
    let blocks = engine.code_blocks("kir");
    assert!(
        !blocks.is_empty(),
        "no ```kir code blocks found in registered pages"
    );

    for block in blocks {
        let report = crate::check_procedure(&block.source);
        assert!(
            report.success,
            "Failed checking kir block in {} at line {}:\n---\n{}\n---\nDiagnostics:\n{:#?}",
            block.path, block.line, block.source, report.diagnostics
        );
    }
}

#[test]
fn documentation_has_no_broken_links() {
    let engine = crate::resources::doc_engine();
    let broken = engine.broken_links();
    assert!(
        broken.is_empty(),
        "Found broken links in documentation:\n{:#?}",
        broken
    );
}

#[test]
fn ir_spec_environment_table_entries_are_valid_ambients() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root");
    let file_path = workspace_root.join("docs/agent/ir-spec.md");
    let content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", file_path.display()));

    let ambients = extract_ambient_table(&content);
    assert!(
        !ambients.is_empty(),
        "no ambient table entries found in docs/agent/ir-spec.md"
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

#[test]
fn ir_spec_builtins_match_compiler_signatures() {
    use karakuri_ir::builtin::Builtin;

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("crates/")
        .parent()
        .expect("workspace root");
    let file_path = workspace_root.join("docs/agent/ir-spec.md");
    let content = std::fs::read_to_string(&file_path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", file_path.display()));

    let mut in_s5 = false;
    let mut tested_builtins = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## 5. Built-in Functions") {
            in_s5 = true;
            continue;
        }
        if in_s5 && trimmed.starts_with("## ") {
            break;
        }
        if in_s5 && trimmed.starts_with("- **") {
            let mut cursor = trimmed;
            while let Some(open) = cursor.find('`') {
                let rest = &cursor[open + 1..];
                let Some(close) = rest.find('`') else { break };
                let item = &rest[..close];
                cursor = &rest[close + 1..];

                let (name, arity) = if let Some((fn_name, args_part)) = item.split_once('(') {
                    let fn_name = fn_name.trim();
                    let args_str = args_part.trim_end_matches(')').trim();
                    let count = if args_str.is_empty() {
                        0
                    } else {
                        args_str.split(',').count()
                    };
                    (fn_name, Some(count))
                } else {
                    (item.trim(), None)
                };

                let builtin = Builtin::from_name(name)
                    .unwrap_or_else(|| panic!("builtin `{name}` in §5 not found in Builtin::ALL"));

                if let Some(expected_args) = arity {
                    let actual_args = builtin.signature().args.len();
                    assert_eq!(
                        actual_args, expected_args,
                        "builtin `{name}` has {actual_args} arguments in signature, but doc specifies {expected_args}"
                    );
                }
                tested_builtins += 1;
            }
        }
    }
    assert!(
        tested_builtins >= 30,
        "expected at least 30 builtins tested from §5, found {tested_builtins}"
    );
}
