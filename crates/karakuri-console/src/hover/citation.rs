//! The console paints its own hover layer, and the words in it are the
//! manual's own.
//!
//! # The words are the page's, and only the key is written down here
//!
//! `docs/manual/console.html` is the only copy of a tip. The page is
//! [`PAGE`], embedded at compile time, and [`Tips::read`] parses the
//! `data-tip` attributes out of it once, at start-up, off the frame path
//! (P-0091). Nothing here restates a word of one, so there is no second copy
//! to drift ([`docs/contributing.md` §4](../../../docs/contributing.md), which
//! is *generated* rather than *tested*).

/// The mock, embedded: the only copy of every tip on this console.
///
/// It is `include_str!` rather than a path read at run time for the reason
/// every other transcription in this crate is a `const`: a panel that had to
/// find `docs/manual/` on disk would draw no tips at all when it was installed
/// anywhere else, and a tip that is missing is indistinguishable from a control
/// that has none.
pub const PAGE: &str = include_str!("../../../../docs/manual/console.html");

/// Where one control's words are in the mock: the element's class, exactly as
/// the page spells it, and the text inside it with its tags removed and its
/// runs of whitespace collapsed.
///
/// It is a citation and not a copy. Nothing here is a word of the tip — what is
/// written down is where to find it, which is the part the page cannot answer
/// for itself.
///
/// The text is in the page's own spelling, entities and all — `&#9662;` stays
/// `&#9662;` — because what is being cited is the markup rather than what a
/// browser makes of it, and a cite a reader can `grep` for is one they can
/// check.
///
/// [`Cite::nth`] is which of the elements matching that pair is meant, and it
/// is 0 for every cite that is unambiguous. A pair that matches nothing, or
/// that matches fewer elements than `nth` reaches, fails in `tests/hover.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cite {
    /// The element's `class` attribute, verbatim, or `""` for an element with none.
    pub class: &'static str,
    /// The text inside the element, tags stripped and whitespace collapsed.
    pub text: &'static str,
    /// Which of the matching elements, in document order, counting from zero.
    pub nth: usize,
}

/// Every tip [`crate::hover::TIPS`] cites, in [`crate::hover::flat`]'s order, read out of [`PAGE`] once.
///
/// An entry is `None` where its [`Cite`] resolved to nothing, which is a
/// transcription that has gone stale rather than a control with no tip — a
/// control with no tip has no row at all. `tests/hover.rs` is where that fails;
/// a panel that met one would draw nothing for that control rather than
/// something wrong.
pub struct Tips {
    words: Vec<Option<String>>,
}

impl Tips {
    /// Parse the page. At start-up and never on a frame: it walks 340 KB once and
    /// allocates one `String` per tip (P-0091).
    pub fn read() -> Tips {
        let elements = elements(PAGE);
        let words = crate::hover::flat()
            .map(|tip| {
                elements
                    .iter()
                    .filter(|el| el.class == tip.cites.class && el.text == tip.cites.text)
                    .nth(tip.cites.nth)
                    .map(|el| decode(el.tip))
            })
            .collect();
        Tips { words }
    }

    /// The words for one control, by its index in [`crate::hover::flat`].
    pub fn get(&self, index: usize) -> Option<&str> {
        self.words.get(index)?.as_deref()
    }

    /// How many tips were asked for, which is [`crate::hover::flat`]'s length.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Whether nothing was asked for at all, which would mean [`crate::hover::TIPS`] is empty —
    /// `Tips` is never empty in this crate and `clippy` asks for this beside
    /// [`Tips::len`].
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Every control whose [`Cite`] resolved to nothing, by index. Empty on a page
    /// and a table that agree.
    pub fn missing(&self) -> impl Iterator<Item = usize> + '_ {
        self.words
            .iter()
            .enumerate()
            .filter(|(_, words)| words.is_none())
            .map(|(index, _)| index)
    }
}

/// One element of the mock that carries a `data-tip`.
pub struct Element<'a> {
    /// Its `class` attribute, or `""`.
    pub class: &'a str,
    /// The text inside it, tags stripped and whitespace collapsed — the page's own
    /// spelling, entities and all.
    pub text: String,
    /// The `data-tip` attribute's value, still escaped.
    pub tip: &'a str,
}

/// Every element in the page carrying a `data-tip`, in document order.
///
/// A scan and not a parser: it finds the attribute, walks back to the `<` that
/// opened the tag, reads the class beside it, and takes the text up to the
/// matching close tag. That is enough for this page and it is deliberately not
/// enough for HTML in general — what it cannot read it drops, and
/// `tests/hover.rs` carries a floor so a scan that stopped reading fails rather
/// than passing over an empty set (`gpu_tests_are_under_mod_gpu.rs`'s shape).
pub fn elements(page: &str) -> Vec<Element<'_>> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = page[from..].find("data-tip=\"") {
        let at = from + found;
        let Some(open) = page[..at].rfind('<') else {
            from = at + 1;
            continue;
        };
        let name_end = page[open + 1..]
            .find(|c: char| c.is_whitespace() || c == '>')
            .map(|end| open + 1 + end)
            .unwrap_or(page.len());
        let name = &page[open + 1..name_end];
        // The opening tag's own end, found with quotes honoured: an attribute
        // value may hold a `>` and the page's `style` attributes do.
        let Some(tag_end) = unquoted(page, name_end, '>') else {
            from = at + 1;
            continue;
        };
        let attrs = &page[name_end..tag_end];
        let class = attribute(attrs, "class").unwrap_or("");
        let tip = attribute(attrs, "data-tip").unwrap_or("");
        let inner = inner_of(page, name, tag_end + 1);
        out.push(Element {
            class,
            text: text_of(inner),
            tip,
        });
        from = tag_end + 1;
    }
    out
}

/// The first `what` outside a quoted attribute value, from `at`.
fn unquoted(page: &str, at: usize, what: char) -> Option<usize> {
    let mut quote = None;
    for (index, ch) in page[at..].char_indices() {
        match (quote, ch) {
            (None, '"') | (None, '\'') => quote = Some(ch),
            (Some(open), ch) if ch == open => quote = None,
            (None, ch) if ch == what => return Some(at + index),
            _ => {}
        }
    }
    None
}

/// One attribute's value out of an opening tag's attribute text.
fn attribute<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let at = attrs.find(&key)? + key.len();
    let end = attrs[at..].find('"')? + at;
    Some(&attrs[at..end])
}

/// The text between an opening tag that ends at `from` and its matching close,
/// with tags of the same name nested inside it counted.
fn inner_of<'a>(page: &'a str, name: &str, from: usize) -> &'a str {
    let open = format!("<{name}");
    let close = format!("</{name}");
    let mut depth = 1usize;
    let mut at = from;
    while depth > 0 {
        let next_open = page[at..].find(&open).map(|found| at + found);
        let next_close = page[at..].find(&close).map(|found| at + found);
        match (next_open, next_close) {
            (Some(o), Some(c)) if o < c => {
                depth += 1;
                at = o + open.len();
            }
            (_, Some(c)) => {
                depth -= 1;
                if depth == 0 {
                    return &page[from..c];
                }
                at = c + close.len();
            }
            _ => return &page[from..],
        }
    }
    &page[from..]
}

/// An element's text: its tags removed and its runs of whitespace collapsed to
/// one space. The page's own entities are left as they are, because a [`Cite`]
/// quotes the markup.
fn text_of(inner: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    let mut space = false;
    for ch in inner.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            _ if inside => {}
            ch if ch.is_whitespace() => space = !out.is_empty(),
            ch => {
                if space {
                    out.push(' ');
                    space = false;
                }
                out.push(ch);
            }
        }
    }
    out
}

/// The named entities the mock uses, and nothing else: an entity that is not
/// here comes out of [`decode`] unchanged and `tests/hover.rs` fails naming it,
/// so the day the page uses a sixteenth this stops reading rather than reading
/// wrongly.
const NAMED: [(&str, &str); 15] = [
    ("&mdash;", "\u{2014}"),
    ("&ndash;", "\u{2013}"),
    ("&middot;", "\u{00b7}"),
    ("&rarr;", "\u{2192}"),
    ("&times;", "\u{00d7}"),
    ("&minus;", "\u{2212}"),
    ("&plusmn;", "\u{00b1}"),
    ("&plus;", "+"),
    ("&hellip;", "\u{2026}"),
    ("&frac12;", "\u{00bd}"),
    ("&lowast;", "\u{2217}"),
    ("&rsquo;", "\u{2019}"),
    ("&nbsp;", "\u{00a0}"),
    ("&lt;", "<"),
    ("&gt;", ">"),
];

/// One tip's words, as a reader sees them: the attribute's value with its
/// entities resolved.
///
/// `&amp;` is last on purpose — it is resolved after every other entity, so a
/// literal ampersand in the page cannot turn the text after it into one.
pub fn decode(escaped: &str) -> String {
    let mut out = String::with_capacity(escaped.len());
    let mut rest = escaped;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let Some(end) = tail.find(';').map(|end| end + 1) else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };
        let entity = &tail[..end];
        match numeric(entity).or_else(|| named(entity)) {
            Some(ch) => out.push_str(&ch),
            None => out.push_str(entity),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out.replace("&amp;", "&")
}

/// `&#8853;` and `&#x2295;`, or `None` for anything else.
fn numeric(entity: &str) -> Option<String> {
    let digits = entity.strip_prefix("&#")?.strip_suffix(';')?;
    let code = match digits
        .strip_prefix('x')
        .or_else(|| digits.strip_prefix('X'))
    {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => digits.parse().ok()?,
    };
    Some(char::from_u32(code)?.to_string())
}

/// One of [`NAMED`], or `None`.
fn named(entity: &str) -> Option<String> {
    NAMED
        .iter()
        .find(|(name, _)| *name == entity)
        .map(|(_, ch)| (*ch).to_owned())
}
