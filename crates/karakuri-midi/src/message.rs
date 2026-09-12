//! What arrived on the wire, as the three things this engine acts on.
//!
//! Deliberately not a general MIDI parser. A control surface sends control
//! changes and notes; everything else on the wire — pitch bend, aftertouch,
//! program change, clock, sysex — is recognised well enough to be *ignored*
//! rather than misread, and that is the whole requirement. A parser that
//! understood more would have more to be wrong about.

/// One message this engine has a use for.
///
/// Channel is `0..16` as it is on the wire, not the `1..17` a controller's
/// front panel prints. The two spellings are one off from each other and both
/// are ubiquitous, so the wire's is the one kept and [`crate::map`] is where the
/// operator's is translated — once, where a human writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// A knob or a fader. `value` is `0..=127`.
    ControlChange {
        channel: u8,
        controller: u8,
        value: u8,
    },
    /// A pad or a key going down. **A note-on at velocity 0 is a note-off**,
    /// and is reported as one: the MIDI spec allows either spelling and
    /// hardware disagrees about which it sends, so a reader that took velocity
    /// 0 for a press would see every release as a second press.
    NoteOn { channel: u8, note: u8, velocity: u8 },
    /// A pad or a key coming up. Carried rather than dropped because a
    /// momentary control — hold to preview, release to go back — is a thing a
    /// surface can do and this is the only message that says it happened.
    NoteOff { channel: u8, note: u8 },
}

impl Message {
    /// Parses a complete raw MIDI byte slice into a [`Message`], or `None` if unhandled or malformed.
    pub fn parse(bytes: &[u8]) -> Option<Message> {
        let (&status, data) = bytes.split_first()?;
        // A data byte where a status byte belongs: running status or a fragment.
        if status & 0x80 == 0 {
            return None;
        }
        let channel = status & 0x0f;
        // Data bytes mask off the top bit (0x7f) per MIDI specification.
        let data = |i: usize| data.get(i).map(|b| b & 0x7f);
        match (status & 0xf0, data(0), data(1)) {
            (0xb0, Some(controller), Some(value)) => Some(Message::ControlChange {
                channel,
                controller,
                value,
            }),
            (0x90, Some(note), Some(0)) => Some(Message::NoteOff { channel, note }),
            (0x90, Some(note), Some(velocity)) => Some(Message::NoteOn {
                channel,
                note,
                velocity,
            }),
            (0x80, Some(note), _) => Some(Message::NoteOff { channel, note }),
            _ => None,
        }
    }

    /// The channel it arrived on, `0..16`.
    pub fn channel(self) -> u8 {
        match self {
            Message::ControlChange { channel, .. }
            | Message::NoteOn { channel, .. }
            | Message::NoteOff { channel, .. } => channel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_control_change_carries_its_channel_controller_and_value() {
        assert_eq!(
            Message::parse(&[0xb3, 7, 100]),
            Some(Message::ControlChange {
                channel: 3,
                controller: 7,
                value: 100,
            })
        );
    }

    /// Verifies that NoteOn with velocity 0 is treated as NoteOff.
    #[test]
    fn a_note_on_at_velocity_zero_is_a_release_not_a_press() {
        assert_eq!(
            Message::parse(&[0x90, 60, 0]),
            Some(Message::NoteOff {
                channel: 0,
                note: 60
            })
        );
        assert_eq!(
            Message::parse(&[0x90, 60, 1]),
            Some(Message::NoteOn {
                channel: 0,
                note: 60,
                velocity: 1
            })
        );
        // And the explicit spelling arrives as the same thing, so a surface
        // that sends one and a surface that sends the other are one case here.
        assert_eq!(
            Message::parse(&[0x80, 60, 64]),
            Message::parse(&[0x90, 60, 0])
        );
    }

    /// Verifies that unsupported message types are ignored and return None.
    #[test]
    fn a_message_this_engine_has_no_use_for_is_none_rather_than_a_near_miss() {
        for bytes in [
            &[0xe0, 0, 64][..],      // pitch bend
            &[0xa0, 60, 64][..],     // polyphonic aftertouch
            &[0xc0, 5][..],          // program change
            &[0xd0, 64][..],         // channel aftertouch
            &[0xf8][..],             // clock
            &[0xf0, 0x7e, 0xf7][..], // sysex
        ] {
            assert_eq!(
                Message::parse(bytes),
                None,
                "{bytes:?} was read as something"
            );
        }
    }

    /// A truncated message is nothing, not a message with a zero in it. A
    /// two-byte control change is a fragment, and reading it as `value = 0`
    /// would pull a fader to silence on a dropped byte.
    #[test]
    fn a_fragment_is_not_a_message_with_a_zero_in_it() {
        assert_eq!(Message::parse(&[]), None);
        assert_eq!(Message::parse(&[0xb0]), None);
        assert_eq!(Message::parse(&[0xb0, 7]), None);
        // And a byte with no status bit is running status or a fragment —
        // either way not this function's to reassemble.
        assert_eq!(Message::parse(&[7, 100]), None);
    }

    /// Verifies that data bytes with top bits set are masked to 7 bits.
    #[test]
    fn a_data_byte_with_its_top_bit_set_is_masked_rather_than_read_wide() {
        assert_eq!(
            Message::parse(&[0xb0, 0x87, 0xe4]),
            Some(Message::ControlChange {
                channel: 0,
                controller: 7,
                value: 100,
            })
        );
        // Including where it would otherwise change what the message *is*: a
        // velocity of 0x80 is 0 once masked, which is a release.
        assert_eq!(
            Message::parse(&[0x90, 60, 0x80]),
            Some(Message::NoteOff {
                channel: 0,
                note: 60
            })
        );
    }

    /// Longer than expected is the message plus something else, and the
    /// message is still the message: a host that delivers a padded buffer
    /// should not lose the fader move in it.
    #[test]
    fn trailing_bytes_do_not_hide_the_message() {
        assert_eq!(
            Message::parse(&[0xb0, 7, 100, 0, 0]),
            Some(Message::ControlChange {
                channel: 0,
                controller: 7,
                value: 100,
            })
        );
    }
}
