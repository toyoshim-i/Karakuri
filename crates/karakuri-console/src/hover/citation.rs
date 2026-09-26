//! Extracts hover tip citations from the manual (`docs/manual/console.html`).
//!
//! Tips are parsed once at startup from compile-time embedded [`PAGE`] (P-0091)
//! to avoid duplication and doc drift.

/// Embedded manual HTML page containing hover tips for console controls.
pub const PAGE: &str = include_str!("../../../../docs/manual/console.html");

/// Locates a control's hover tip in the manual by HTML class, inner text, and index.
///
/// Preserves exact markup spelling and entities to allow verified lookup in tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cite {
    /// The element's `class` attribute, verbatim, or `""` for an element with none.
    pub class: &'static str,
    /// The text inside the element, tags stripped and whitespace collapsed.
    pub text: &'static str,
    /// Which of the matching elements, in document order, counting from zero.
    pub nth: usize,
}

/// Cached tip strings resolved from [`PAGE`] matching [`crate::hover::TIPS`].
///
/// An entry is `None` if its [`Cite`] could not be resolved from markup.
pub struct Tips {
    words: Vec<Option<String>>,
}

impl Tips {
    /// Parses tips from the embedded manual page (P-0091).
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

    /// Returns the decoded tip text for the specified probe index.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.words.get(index)?.as_deref()
    }

    /// Returns the total number of registered tips.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Returns true if no tips are registered.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Returns an iterator of indices whose [`Cite`] failed to resolve.
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

/// Scans document-order HTML elements bearing a `data-tip` attribute.
///
/// Extracts the class and inner text between tags for tip resolution.
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
        // Opening tag end respecting quoted attribute values.
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

/// Strips markup tags and collapses whitespace, preserving HTML entities.
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

/// Named HTML entities supported from the manual markup.
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

/// Decodes HTML entities in tip text, resolving `&amp;` last to avoid double-unescaping.
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
