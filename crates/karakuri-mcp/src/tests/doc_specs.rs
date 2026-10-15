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

#[test]
fn all_refusal_and_diagnostic_codes_have_documentation_pages() {
    use karakuri_ir::error::codes;
    use karakuri_operation::RefusalCode;

    let engine = crate::resources::doc_engine();
    let all_codes: Vec<&str> = RefusalCode::ALL
        .iter()
        .map(|c| c.as_str())
        .chain(codes::ALL.iter().copied())
        .collect();
    let missing = engine.missing_pages(&all_codes);
    assert!(
        missing.is_empty(),
        "The following refusal or diagnostic codes lack documentation pages in docs/agent/rules/: {missing:#?}"
    );
}

#[test]
fn no_raw_kir_e_literals_remain_outside_code_definitions() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_crates = manifest_dir.parent().expect("crates dir");
    let mut violations = Vec::new();

    fn scan_dir(dir: &Path, violations: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                if name != "target" && !name.starts_with('.') {
                    scan_dir(&path, violations);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let path_str = path.to_string_lossy();
                // Exclude this test file itself
                if path_str.ends_with("crates/karakuri-mcp/src/tests/doc_specs.rs") {
                    continue;
                }
                let Ok(content) = std::fs::read_to_string(&path) else {
                    continue;
                };
                for (line_no, line) in content.lines().enumerate() {
                    if line.contains("\"KIR-E") {
                        // Only allowed in crates/karakuri-ir/src/error.rs inside pub mod codes definitions
                        if path_str.ends_with("crates/karakuri-ir/src/error.rs")
                            && line.trim().starts_with("pub const E")
                        {
                            continue;
                        }
                        violations.push(format!("{path_str}:{}: {line}", line_no + 1));
                    }
                }
            }
        }
    }

    scan_dir(workspace_crates, &mut violations);
    assert!(
        violations.is_empty(),
        "Raw KIR-E string literals found outside codes definitions:\n{}",
        violations.join("\n")
    );
}

#[test]
fn refusal_and_diagnostic_reports_include_mcp_doc_read_hints() {
    use karakuri_ir::error::codes;
    use karakuri_operation::RefusalCode;

    let detail = karakuri_operation::RefusalDetail {
        code: RefusalCode::SlotInMix,
        message: "target slot is active in live mix".to_string(),
        slot: Some(1),
        deck: Some(1),
        lane: None,
        class: None,
        policy: None,
        in_mix: Some(true),
    };
    let payload = crate::tools::refusal_payload(&detail);
    let val: serde_json::Value = serde_json::from_str(&payload).expect("json payload");
    assert_eq!(
        val["hint"].as_str(),
        Some("Help: call tool 'doc_read {\"id\": \"SLOT_IN_MIX\"}'")
    );

    let res = crate::tools::tool_result(Err(payload));
    let serialized = serde_json::to_value(&res).expect("tool_result json");
    let text = serialized["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("Help: call tool 'doc_read {\"id\": \"SLOT_IN_MIX\"}'"));

    // Also verify diagnostic report hint attachment
    let diag_report = karakuri_ir::DiagnosticReport {
        diagnostics: vec![karakuri_ir::Diagnostic::new(
            codes::E100_PARSE,
            "syntax error",
            Some(1),
            Some(1),
            None,
        )],
        success: false,
        hash: None,
        hint: None,
    };
    let diag_res = crate::tools::tool_result(Err(serde_json::to_string(&diag_report).unwrap()));
    let diag_val = serde_json::to_value(&diag_res).expect("diag tool_result json");
    let expected_hint = format!(
        "Help: call tool 'doc_read {{\"id\": \"{}\"}}'",
        codes::E100_PARSE
    );
    assert_eq!(
        diag_val["structuredContent"]["report"]["hint"].as_str(),
        Some(expected_hint.as_str())
    );
    assert_eq!(
        diag_val["structuredContent"]["report"]["diagnostics"][0]["hint"].as_str(),
        Some(expected_hint.as_str())
    );
}
