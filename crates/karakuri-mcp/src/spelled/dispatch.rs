use serde_json::{json, Value};

use super::*;

/// Every operation `operate` takes, in the manual's order.
pub(crate) fn operable() -> Vec<&'static Spelled> {
    SPELLED.iter().filter(|row| row.make.is_some()).collect()
}

/// The row for one heading, or `None` where the vocabulary does not carry it.
pub(crate) fn spelled_named(title: &str) -> Option<&'static Spelled> {
    SPELLED.iter().find(|row| row.title() == title)
}

/// The heading nearest to a name this vocabulary does not carry.
///
///
/// [P-0083](../../../docs/principles/0083-a-refusal-carries-what-the-next-attempt-needs.md):
/// a model that misremembers a heading by one word gets the heading back rather
/// than a list of sixty-four to search. The measure is how many words the name
/// and the heading share, with the closest length for a tie, and it is
/// deliberately not an edit distance: the mistakes here are whole words rather
/// than letters.
///
/// The joining words are dropped, and matching whole words is the point.
/// Matched as substrings, *set the gain* comes back as *Reset the arrangement*
/// — `the` is in both and `set` is inside `Reset` — which is a confident wrong
/// answer of exactly the kind
/// [P-0084](../../../docs/principles/0084-a-confident-wrong-automatic-judgement-is-worse-than-not-judging.md)
/// is about.
pub(crate) fn nearest(said: &str) -> &'static str {
    /// The words that say nothing about which operation is meant.
    const COMMON: [&str; 16] = [
        "the", "a", "an", "of", "to", "in", "on", "is", "it", "and", "or", "what", "which", "one",
        "at", "for",
    ];
    fn parts(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty() && !COMMON.contains(word))
            .map(str::to_string)
            .collect()
    }
    let asked = parts(said);
    let mut best = ("", 0usize, usize::MAX);
    for title in Operation::TITLES {
        let held = parts(title);
        let shared = asked.iter().filter(|word| held.contains(word)).count();
        let apart = title.len().abs_diff(said.len());
        if shared > best.1 || (shared == best.1 && apart < best.2) {
            best = (title, shared, apart);
        }
    }
    best.0
}

/// One `operate` call as the operation it names, or the refusal it earned.
///
/// The two halves are a name and a payload, and they are refused in that order
/// for [`asked`]'s reason: a call with two mistakes in it is told about the one
/// a reader would fix first.
pub(crate) fn operated(args: &Value, slots: &Slots) -> Result<Operation, String> {
    let named = args.get("operation").and_then(Value::as_str).ok_or(
        "`operation` is required and is an operation's own heading, spelled exactly as \
             `docs/manual/operations.html` writes it — this tool's `operation` list is every \
             one it takes",
    )?;
    let Some(row) = spelled_named(named) else {
        return Err(format!(
            "no operation `{named}` — the nearest heading this vocabulary carries is \
             `{}`. Every name this tool takes is in its own `operation` list, and \
             `karakuri://operations` is that list with each payload's shape beside it",
            nearest(named)
        ));
    };
    let Some(make) = row.make else {
        let (operation, _) = (row.sample)();
        return Err(match sayable(&operation) {
            // Cannot happen: `make` is `Some` for exactly the operable rows, and
            // `the_table_and_the_classification_agree` is what says so. Written
            // out rather than left to a wildcard for [`absent`]'s reason.
            Sayable::Operable => format!(
                "`{named}` is an operation this tool takes and this server has no spelling \
                 for, which is a fault in the server rather than in the call"
            ),
            Sayable::Tool(tool) => format!(
                "`{named}` is reached over MCP by `{tool}` rather than by `operate`: it does \
                 something only this server can do — a file, the store, a listing — and a \
                 second spelling of a tool is a second spelling. Call `{tool}`"
            ),
            Sayable::Window => format!(
                "`{named}` is a surface's own state, and a route into a surface's own state \
                 is a route into a window a model is not looking at. Nothing here can ask \
                 for it, and the person at the panel is who it belongs to"
            ),
            Sayable::Never(why) => format!(
                "`{named}` is named by this vocabulary and has no route on this surface: \
                 {why}"
            ),
        });
    };
    make(&payload(args), slots)
}

/// The `operate` tool, generated from [`SPELLED`].
///
/// The `operation` list is every heading this surface takes, in the manual's
/// order, and the payloads are described under it rather than as one `oneOf`:
/// what a client needs before it calls is *which names there are* and *what
/// each one takes*, and a schema that expressed the second as a union of thirty
/// objects would be read by nothing and understood by no one. The shapes go to
/// `karakuri://operations`, which is the same table rendered
/// ([ADR-0092](../../../docs/adr/0092-a-resource-listing-is-a-curriculum.md)).
pub(crate) fn operate_tool() -> Value {
    let names: Vec<&'static str> = operable().iter().map(|row| row.title()).collect();
    json!({
        "name": "operate",
        "description":
            "Ask for one operation of this instrument by its own name. The names are the \
             headings of `docs/manual/operations.html`, which is the one vocabulary every \
             surface routes into — the panel, the keyboard, a MIDI map and this server all \
             name the same things, so an operation asked for here is performed where a hand \
             on the panel would have performed it, on the next frame. Read \
             `karakuri://operations` for the payload each name takes.\n\n\
             Operations that could stop a performance are refused until the operator opens \
             their class at the panel. The refusal says which class it is in and where the \
             operator opens it, so it can be handed to the person sitting there. The list \
             above never shortens: an operation is connected whether or not its class is \
             open, and the answer is a refusal rather than a missing tool.\n\n\
             The seven tools beside this one are not spelled here. Each of them does \
             something only this server can do, and `operate` names the tool instead.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": names,
                    "description": "the operation's own heading, verbatim",
                },
                "with": {
                    "type": "object",
                    "description":
                        "the payload, whose shape follows the operation — \
                         `karakuri://operations` has one schema per name. Absent where the \
                         operation takes none",
                },
            },
            "required": ["operation"],
        },
    })
}

/// Every operation this surface takes, with its payload's shape — the
/// curriculum a client reads before it calls, generated from [`SPELLED`] rather
/// than written down beside it.
pub(crate) fn operations() -> String {
    let mut out = String::from(
        "# Operations `operate` takes\n\n\
         Generated from this server's own table, so this is exactly what will be \
         accepted. Each heading is the name to put in `operation`, and the schema under \
         it is the `with` object.\n\n\
         An operation whose class the operator has not opened is **refused**, and the \
         refusal says which class and where it opens. That is not a reason to avoid \
         calling it: the refusal is what tells the person at the panel what to open.\n\n",
    );
    for row in operable() {
        let (_, call) = (row.sample)();
        let shape = (row.shape).expect("an operable row has a shape")();
        out.push_str(&format!(
            "## {}\n\n```json\n{}\n```\n\nOne call:\n\n```json\n{}\n```\n\n",
            row.title(),
            serde_json::to_string_pretty(&shape).unwrap_or_else(|_| shape.to_string()),
            serde_json::to_string_pretty(&json!({ "operation": row.title(), "with": call }))
                .unwrap_or_default(),
        ));
    }
    out
}
