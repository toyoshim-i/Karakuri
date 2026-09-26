use super::*;

// -- the surface against the vocabulary and the page -------------------

/// The specification, relative to the workspace root — the same page
/// `karakuri-operation`'s `the_manual_and_the_vocabulary_agree.rs` and
/// `karakuri-console`'s `tests/vocabulary.rs` read, and this is that check for
/// the third surface.
const PAGE: &str = "docs/manual/operations.html";

/// What marks an operation on that page. Every row opens with this div and
/// nothing else on the page uses it; sections are `<h2>` and the legend is
/// neither. The same marker both other tests match, for their reason.
const ROW: &str = r#"<div class="op-head">"#;

fn page() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(PAGE);
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "{} is the specification and could not be read: {e}",
            path.display()
        )
    })
}

/// Every row's title and its MCP badge in page order (`has`, `plan`, or `gap`).
fn mcp_routes() -> Vec<(String, String, String)> {
    let html = page();
    let mut found = Vec::new();
    for row in html.split(ROW).skip(1) {
        let Some(open) = row.find("<h3>") else {
            continue;
        };
        let rest = &row[open + "<h3>".len()..];
        let Some(close) = rest.find("</h3>") else {
            continue;
        };
        let title = rest[..close].to_string();
        // The row ends where the next section does; a badge found past that
        // would belong to another row.
        let body = &rest[close..];
        let body = &body[..body.find("</section>").unwrap_or(body.len())];
        let Some(at) = body.find(r#"<span class="rt "#) else {
            continue;
        };
        let mut badge = None;
        for span in body[at..].split(r#"<span class="rt "#).skip(1) {
            let Some(quote) = span.find('"') else {
                continue;
            };
            let class = span[..quote].to_string();
            let Some(text) = span[quote..].strip_prefix(r#"">MCP <b>"#) else {
                continue;
            };
            let Some(shut) = text.find("</b>") else {
                continue;
            };
            badge = Some((class, text[..shut].to_string()));
            break;
        }
        let Some((class, names)) = badge else {
            continue;
        };
        found.push((title, class, names));
    }
    found
}

/// Returns minimal valid argument payloads satisfying each published tool schema.
fn sample(name: &str) -> Value {
    match name {
        "read_procedure" => json!({ "slot": 0, "layer": "L4" }),
        "write_procedure" => json!({ "slot": 0, "layer": "L4", "source": "" }),
        "wire_input" => json!({ "slot": 0, "node": "warp", "input": "shape", "to": "blob" }),
        "swap_outcome" => json!({}),
        "save_set" => json!({ "slot": 0 }),
        "read_set" => json!({ "id": "a_set" }),
        "list_sets" => json!({}),
        "walk_history" => json!({ "set": "a_set" }),
        // Test the single open operation that passes audit validation.
        "operate" => json!({
            "operation": "Put a node's previous version back",
            "with": { "deck": 0, "revision": { "previous": { "layer": "L4" } } },
        }),
        other => panic!(
            "`{other}` is published by `tools()` and this file has no arguments for it — \
             add the smallest call that gets past its schema, so the survey below reaches \
             it rather than passing over it"
        ),
    }
}

const WORKFLOW_TOOLS: &[&str] = &[
    "get_permissions",
    "read_slot",
    "copy_slot",
    "check_procedure",
    "check_set",
];

/// Every tool this server publishes, with the operation one call names.
fn published() -> Vec<(String, Operation)> {
    let slots = slots();
    tools()
        .as_array()
        .expect("tools() is an array")
        .iter()
        .filter_map(|tool| {
            let name = tool["name"]
                .as_str()
                .expect("a tool has a name")
                .to_string();
            if WORKFLOW_TOOLS.contains(&name.as_str()) {
                return None;
            }
            let asked = asked(&name, &sample(&name), &slots)
                .unwrap_or_else(|e| panic!("`{name}` is advertised and is not a tool: {e}"));
            match asked {
                Asked::Named(operation) => Some((name, operation)),
                Asked::Refused(refusal) => panic!(
                    "`{name}` refused the sample call in this file: {refusal} — the \
                     arguments in `sample` no longer get past its schema"
                ),
            }
        })
        .collect()
}

#[test]
fn every_workflow_tool_is_published_by_tools() {
    let names: std::collections::HashSet<String> = tools()
        .as_array()
        .expect("tools() is an array")
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    for tool in WORKFLOW_TOOLS {
        assert!(
            names.contains(*tool),
            "workflow tool `{tool}` must be in tools()"
        );
    }
}

/// Verifies that every published MCP tool corresponds to an operation documented on the manual page.
#[test]
fn every_tool_this_server_publishes_has_a_route_on_the_page() {
    let routes = mcp_routes();
    assert!(
        routes.len() >= 50,
        "only {} rows with an MCP badge found in {PAGE} — is a row still `{ROW}` \
         followed by an `<h3>` and four `rt` badges? A scan that matched nothing would \
         pass every assertion below",
        routes.len()
    );
    let published = published();
    assert!(
        published.len() >= 7,
        "only {} tools published — this server has fewer than the page's MCP column \
         claims",
        published.len()
    );
    for (name, operation) in &published {
        let title = operation.title();
        let row = routes
            .iter()
            .find(|(row, _, _)| row == title)
            .unwrap_or_else(|| {
                panic!(
                    "`{name}` names `{title}` and {PAGE} has no row with that heading — a \
                     tool reaching an operation nobody specified. The page is the \
                     specification, so add the row there first"
                )
            });
        assert_eq!(
            row.1, "has",
            "`{name}` names `{title}`, which {PAGE} marks `{}` for MCP — a tool that \
             exists and a page that says it does not",
            row.1
        );
        assert_eq!(
            row.2, *name,
            "`{title}` is marked as reached over MCP by `{}`, and the tool that names \
             that operation is `{name}` — the page names a call nobody can make",
            row.2
        );
    }
}

/// Verifies that a `has` badge corresponds to a valid existing tool route.
#[test]
fn every_mcp_route_the_page_claims_is_a_tool_this_server_publishes() {
    let routes = mcp_routes();
    let claimed: Vec<&(String, String, String)> = routes
        .iter()
        .filter(|(_, class, _)| class == "has")
        .collect();
    assert!(
        claimed.len() >= 6,
        "only {} rows of {PAGE} claim an MCP route — the scan found less than the \
         column holds, which would pass this test by finding nothing",
        claimed.len()
    );
    let published = published();
    for (title, _, names) in claimed {
        // Validates `operate` against operation spelling rather than individual operations.
        if names == "operate" {
            let row = spelled_named(title).unwrap_or_else(|| {
                panic!(
                    "{PAGE} says `{title}` is reached over MCP by `operate`, and this \
                     vocabulary carries no operation with that heading"
                )
            });
            assert!(
                row.make.is_some(),
                "{PAGE} says `{title}` is reached over MCP by `operate`, and `operate` \
                 refuses that name — the page claims a route a model cannot take"
            );
            continue;
        }
        let tool = published
            .iter()
            .find(|(name, _)| name == names)
            .unwrap_or_else(|| {
                panic!(
                    "{PAGE} says `{title}` is reached over MCP by `{names}`, and this \
                     server publishes no such tool — the page claims a route a model \
                     cannot take. Either the tool went and the badge is now `gap`, or it \
                     was renamed on the wire"
                )
            });
        assert_eq!(
            tool.1.title(),
            title,
            "{PAGE} says `{title}` is reached by `{names}`, and `{names}` names \
             `{}` — one operation on the page and another in the server",
            tool.1.title()
        );
    }
}

/// Verifies that all operations accepted by spelling are marked `has operate`.
#[test]
fn every_operation_operate_takes_stands_where_the_page_says_it_does() {
    let routes = mcp_routes();
    assert!(
        routes.len() >= 50,
        "only {} rows with an MCP badge found in {PAGE}",
        routes.len()
    );
    for row in SPELLED.iter() {
        let title = row.title();
        let (badge, names) = routes
            .iter()
            .find(|(heading, _, _)| heading == title)
            .map(|(_, class, names)| (class.as_str(), names.as_str()))
            .unwrap_or_else(|| panic!("{PAGE} has no row headed `{title}`"));
        match row.make {
            Some(_) => assert_eq!(
                (badge, names),
                ("has", "operate"),
                "`operate` takes `{title}` and {PAGE} marks it `{badge}` naming \
                 `{names}` — a route a model can take that the page does not describe"
            ),
            None => assert!(
                names != "operate",
                "{PAGE} says `{title}` is reached by `operate`, and `operate` refuses \
                 it: {}",
                match sayable(&(row.sample)().0) {
                    Sayable::Tool(tool) => format!("`{tool}` is its tool"),
                    Sayable::Window => "a model has no window".to_string(),
                    Sayable::Never(why) => why.to_string(),
                    Sayable::Operable => "it does not".to_string(),
                }
            ),
        }
    }
}

/// Verifies that published tools follow expected recording behavior (Silent, OnLanding, NoRecord).
#[test]
fn no_tool_writes_a_record_where_it_is_asked() {
    use karakuri_operation_record::{Current, Silent, Written};
    for (name, operation) in published() {
        let written = karakuri_operation_record::written(&operation, &Current::default());
        let expected = match name.as_str() {
            "read_procedure" | "read_set" | "list_sets" | "walk_history" | "swap_outcome" => {
                Silent::Question
            }
            "save_set" | "write_procedure" => Silent::OnLanding,
            "wire_input" => Silent::NoRecord,
            "operate" => Silent::OnLanding,
            other => {
                panic!("`{other}` is published and this test does not know what it writes")
            }
        };
        assert_eq!(
            written,
            Written::Silent(expected),
            "`{name}` names `{}`, and what it writes is no longer `{expected:?}`",
            operation.title()
        );
    }
}
