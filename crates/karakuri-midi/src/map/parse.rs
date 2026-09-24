use super::*;

pub(crate) fn parse_line(line: &str) -> Result<(Key, Entry), String> {
    let (from, to) = line
        .split_once("->")
        .ok_or_else(|| "expected `<message> -> <control>`".to_string())?;
    let (key, lsb) = parse_key(from.trim())?;
    let target = parse_target(to.trim())?;
    let is_note = matches!(key, Key::Note { .. });
    if is_note && target.continuous() {
        return Err("a note is a press, and this control takes a position; map a `cc`".to_string());
    }
    if !is_note && !target.continuous() {
        return Err(
            "a control change is a position, and this control takes a press; map a `note`"
                .to_string(),
        );
    }
    Ok((key, Entry { target, lsb }))
}

fn parse_key(from: &str) -> Result<(Key, Option<u8>), String> {
    let mut words = from.split_whitespace();
    let kind = words
        .next()
        .ok_or_else(|| "expected `cc`, `cc14` or `note`".to_string())?;
    let number: u8 = words
        .next()
        .ok_or_else(|| format!("`{kind}` needs a number"))?
        .parse()
        .map_err(|_| format!("`{kind}` needs a number in 0-127"))?;
    if number > 127 {
        return Err(format!("`{kind} {number}` is past 127"));
    }
    // Explicitly parse paired LSB controller for cc14 lines.
    let mut lsb = None;
    let mut after = words.next();
    if kind == "cc14" {
        let word = after.filter(|word| *word != "ch").ok_or_else(|| {
            format!(
                "`cc14 {number}` needs the controller its LSB half arrives on — write \
                 `cc14 {number} {}`, which is the usual pairing",
                u16::from(number) + 32
            )
        })?;
        let fine: u8 = word.parse().map_err(|_| {
            format!("`cc14 {number} {word}`: the LSB half is a controller number in 0-127")
        })?;
        if fine > 127 {
            return Err(format!("`cc14 {number} {fine}`: the LSB half is past 127"));
        }
        if fine == number {
            return Err(format!(
                "`cc14 {number} {fine}`: the two halves are one controller — an MSB and its LSB \
                 are two"
            ));
        }
        lsb = Some(fine);
        after = words.next();
    }
    let channel = match (after, words.next()) {
        (None, _) => None,
        (Some("ch"), Some(n)) => {
            // The front panel's spelling, 1-16, because that is what is printed
            // on the device an operator is reading it off. The wire's 0-15 is
            // `Message`'s and the translation happens here, once.
            let n: u8 = n
                .parse()
                .map_err(|_| format!("`ch {n}` needs a channel in 1-16"))?;
            if !(1..=16).contains(&n) {
                return Err(format!("`ch {n}` is outside 1-16"));
            }
            Some(n - 1)
        }
        (Some(other), _) => return Err(format!("expected `ch <1-16>`, found `{other}`")),
    };
    if words.next().is_some() {
        return Err("too many words before `->`".to_string());
    }
    Ok(match kind {
        "cc" | "cc14" => (
            Key::Cc {
                channel,
                controller: number,
            },
            lsb,
        ),
        "note" => (
            Key::Note {
                channel,
                note: number,
            },
            None,
        ),
        other => return Err(format!("`{other}` is not `cc`, `cc14` or `note`")),
    })
}

fn parse_target(to: &str) -> Result<Target, String> {
    let (words, range) = split_range(to)?;
    let mut words = words.split_whitespace();
    let name = words
        .next()
        .ok_or_else(|| "expected a control after `->`".to_string())?;
    let slot = |words: &mut std::str::SplitWhitespace| -> Result<DeckSlot, String> {
        let n = words
            .next()
            .ok_or_else(|| format!("`{name}` needs a slot number"))?;
        n.parse::<u8>()
            .map(DeckSlot::from)
            .map_err(|_| format!("`{name} {n}`: expected a slot number"))
    };
    let target = match name {
        "gain" => Target::Gain {
            slot: slot(&mut words)?,
            range: range.unwrap_or(GAIN_RANGE),
        },
        "opacity" => Target::Opacity {
            slot: slot(&mut words)?,
            range: range.unwrap_or(OPACITY_RANGE),
        },
        "exposure" => Target::Exposure {
            range: range.unwrap_or(EXPOSURE_RANGE),
        },
        // Mask position along the transition axis (ADR-0202).
        "mask-position" => Target::MaskPosition {
            slot: slot(&mut words)?,
            range: range.unwrap_or(MASK_POSITION_RANGE),
        },
        "residency" => {
            let slot = slot(&mut words)?;
            Target::Residency {
                slot,
                residency: value_word(
                    words.next(),
                    Residency::ALL,
                    Residency::name,
                    &format!("residency {slot}"),
                    "a state",
                )?,
            }
        }
        "blend" => {
            let slot = slot(&mut words)?;
            Target::Blend {
                slot,
                blend: value_word(
                    words.next(),
                    BlendMode::ALL,
                    BlendMode::name,
                    &format!("blend {slot}"),
                    "a mode",
                )?,
            }
        }
        "tap" => Target::Tap,
        // Published Set parameter index (1-indexed matching the Inspector UI).
        "param" => {
            let slot = slot(&mut words)?;
            let n = words
                .next()
                .ok_or_else(|| format!("`param {slot}` needs a position in the interface"))?;
            let position: u16 = n
                .parse()
                .map_err(|_| format!("`param {slot} {n}`: expected a position"))?;
            if position == 0 {
                return Err(format!(
                    "`param {slot} 0`: positions count from one, which is the number the \
                     Inspector draws beside the row"
                ));
            }
            Target::Param {
                slot,
                position,
                range,
            }
        }
        // Reject deprecated relative-toggle keywords (`on-air`, `prime`) with actionable migration guidance.
        "on-air" | "prime" => {
            let n = words.next().unwrap_or("N");
            let (flipped, asked) = if name == "on-air" {
                ("a deck on and off", "live")
            } else {
                ("a request on and off", "priming")
            };
            return Err(format!(
                "`{name} {n}` flipped {flipped} rather than naming where it goes; write \
                 `residency {n} {asked}` or `residency {n} allocated`"
            ));
        }
        other => {
            return Err(format!(
                "`{other}` is not a control — expected gain, opacity, exposure, \
                 mask-position, param, residency, blend or tap"
            ))
        }
    };
    if words.next().is_some() {
        return Err(format!("`{name}` takes no more words"));
    }
    if range.is_some() && !target.continuous() {
        return Err(format!("`{name}` is a press and has no range"));
    }
    if target.shape() == Shape::Ratio {
        let [lo, hi] = range.unwrap_or(EXPOSURE_RANGE);
        if lo <= 0.0 || hi <= 0.0 {
            return Err(format!(
                "`{name}` is a ratio control, so its range cannot reach zero"
            ));
        }
    }
    if let Some([lo, hi]) = range {
        if !(lo.is_finite() && hi.is_finite()) || lo == hi {
            return Err(format!("`{name}`: a range needs two different finite ends"));
        }
    }
    Ok(target)
}

/// Parses a token against a predefined slice of vocabulary values, returning an error naming all alternatives if invalid.
fn value_word<T: Copy, const N: usize>(
    word: Option<&str>,
    all: [T; N],
    name: fn(T) -> &'static str,
    line: &str,
    what: &str,
) -> Result<T, String> {
    let listed = || {
        all.iter()
            .map(|v| format!("`{line} {}`", name(*v)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let Some(word) = word else {
        return Err(format!("`{line}` needs {what} — write one of {}", listed()));
    };
    all.iter()
        .copied()
        .find(|v| name(*v) == word)
        .ok_or_else(|| format!("`{line} {word}` is not {what} — write one of {}", listed()))
}

/// Split a trailing `[lo, hi]` off a control, if there is one.
fn split_range(to: &str) -> Result<(&str, Option<[f32; 2]>), String> {
    let Some(open) = to.find('[') else {
        return Ok((to, None));
    };
    let rest = &to[open..];
    let close = rest
        .find(']')
        .ok_or_else(|| "a range opened with `[` and did not close".to_string())?;
    let inside = &rest[1..close];
    if !rest[close + 1..].trim().is_empty() {
        return Err("a range has to be the last thing on the line".to_string());
    }
    let mut ends = inside.split(',');
    let parse = |s: Option<&str>| -> Result<f32, String> {
        s.ok_or_else(|| "a range is `[lo, hi]`".to_string())?
            .trim()
            .parse::<f32>()
            .map_err(|_| "a range is `[lo, hi]`, with numbers".to_string())
    };
    let lo = parse(ends.next())?;
    let hi = parse(ends.next())?;
    if ends.next().is_some() {
        return Err("a range is `[lo, hi]`, and no more".to_string());
    }
    Ok((&to[..open], Some([lo, hi])))
}
