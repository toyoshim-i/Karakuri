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

/// Every row's title and its MCP badge, in page order: the badge's class —
/// `has`, `plan` or `gap` — and the text it names the route with.
///
/// Read verbatim and never decoded, exactly as the vocabulary's own test reads
/// a heading: a `gap` badge says `&mdash;`, and a tool name that needed
/// decoding to match would be a tool nobody could type.
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
        // **The one operation `operate` names that the audit lets through**,
        // which is what makes this survey mean the same thing for the eighth
        // tool as it does for the seven: the other twenty-nine are refused
        // by the gate by design, and a sample drawn from those would make
        // *every tool names an operation the gate lets through* false about
        // a tool that is working exactly as ADR-0235 says it should. The
        // rows that are closed are surveyed by
        // `every_operation_operate_takes_stands_where_the_page_says_it_does`
        // instead.
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

/// Every tool this server publishes, with the operation one call names.
fn published() -> Vec<(String, Operation)> {
    let slots = slots();
    tools()
        .as_array()
        .expect("tools() is an array")
        .iter()
        .map(|tool| {
            let name = tool["name"]
                .as_str()
                .expect("a tool has a name")
                .to_string();
            let asked = asked(&name, &sample(&name), &slots)
                .unwrap_or_else(|e| panic!("`{name}` is advertised and is not a tool: {e}"));
            match asked {
                Asked::Named(operation) => (name, operation),
                Asked::Refused(refusal) => panic!(
                    "`{name}` refused the sample call in this file: {refusal} — the \
                     arguments in `sample` no longer get past its schema"
                ),
            }
        })
        .collect()
}

/// A tool with no row is an operation nobody specified.
///
/// The page is the specification for which operations exist — that is what
/// `karakuri-operation`'s own manual test is built on — so a tool reaching
/// something the page does not name would be this surface inventing an
/// operation, with no prose and no other three routes.
///
/// The row is matched on the operation's title, which comes from [`asked`]
/// rather than from a table here, and on the badge's own text, which has to
/// name the tool: a row marked `has` that named a different tool would be a
/// route the page describes and nobody can call.
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

/// The other direction: a `has` badge with no tool is the page claiming a route
/// that does not exist.
///
/// It fails apart from the test above because it is a different failure: that
/// one says the surface reached past the specification, this one says the
/// specification promises a model something it cannot do.
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
        // **`operate` is checked against the spelling rather than against
        // one operation**, which is the difference between the eighth tool
        // and the seven: a tool of its own names one row, and `operate`
        // names every row the spelling takes. So the page claiming
        // `operate` on a row is checked by asking [`SPELLED`] whether it
        // takes that row's operation — the same question a call asks.
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

/// Every operation the spelling takes has a row marked `has operate`, and every
/// row that is not marked so is one the spelling refuses.
///
/// The other direction of the test above, and the one that catches the silent
/// half: a row `operate` reaches whose badge still reads `plan` is a route a
/// model can take and the page does not describe, which nothing else here would
/// notice.
#[test]
fn every_operation_operate_takes_stands_where_the_page_says_it_does() {
    let routes = mcp_routes();
    assert!(
        routes.len() >= 50,
        "only {} rows with an MCP badge found in {PAGE}",
        routes.len()
    );
    for row in SPELLED {
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

/// Not one of the seven writes a record where it is asked, which is why none of
/// them routes through `Live::operate` and why this module performs its own.
///
/// And one of them writes no record at all, which is a hole this test pins
/// rather than blesses. `wire_input` answers `NoRecord` because `Record::Edge`
/// is a Set file's record with no `slot` to carry the deck `WireInput` names —
/// so a rewiring during a set is the one thing a model can do on this surface
/// that a replay does not reconstruct. It is asserted here so that the day
/// `Record::Edge` grows a `slot` and `written` answers with it, this fails and
/// names the tool whose answer has changed.
///
/// Asserted against `karakuri-operation-record` rather than against this file,
/// in the shape ADR-0198 gave the key handler's owed list: the day one of these
/// conversions changes — a `read_set` that logged, a `save_set` whose record
/// moved off the landing frame — the failure names the tool that is due to move
/// rather than leaving this surface performing something the record layer has
/// since taken over.
#[test]
fn no_tool_writes_a_record_where_it_is_asked() {
    use karakuri_operation_record::{Current, Silent, Written};
    for (name, operation) in published() {
        let written = karakuri_operation_record::written(&operation, &Current::default());
        let expected = match name.as_str() {
            // It asks rather than changes, and a question writes no record.
            //
            // **`walk_history` is here on the day it arrived**, which is
            // the whole of why it is a tool: what it does is a listing of
            // the store, and `written` says so rather than this file
            // asserting it (`docs/adr/0342-…`). Landing one of the rows it
            // returns is `RestoreProcedure`, which is `OnLanding` two arms
            // down and reaches the frame through `operate`.
            "read_procedure" | "read_set" | "list_sets" | "walk_history" | "swap_outcome" => {
                Silent::Question
            }
            // Its record is written where the work lands: `Record::Save` at
            // the frame the save landed, `Record::Procedure` when a swap
            // lands.
            "save_set" | "write_procedure" => Silent::OnLanding,
            // **Nothing carries it**, which is the hole named above and not
            // a question this tool asks or work it lands.
            "wire_input" => Silent::NoRecord,
            // **`operate` is the tool this claim is not about**, and saying
            // so is the point rather than an exception. The seven perform
            // themselves *because* they write nothing where they are asked;
            // `operate` performs nothing and hands the operation to the
            // frame the panel performs presses on, so whatever it writes is
            // written there, by the same `written` this asserts against, at
            // the same instant a press of it would write. The sample here is
            // `RestoreProcedure`, whose `Record::Procedure` lands at the
            // swap.
            "operate" => Silent::OnLanding,
            other => {
                panic!("`{other}` is published and this test does not know what it writes")
            }
        };
        assert_eq!(
            written,
            Written::Silent(expected),
            "`{name}` names `{}`, and what it writes is no longer `{expected:?}` — this \
             surface performs it here because there was no record to route into, and \
             that is what has changed",
            operation.title()
        );
    }
}
