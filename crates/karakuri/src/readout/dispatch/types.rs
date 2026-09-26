use karakuri_layout::Point;
use karakuri_operation::Operation;

use super::Outcome;

/// Pointer input event simplified for console interaction routing (ADR-0307, ADR-0311).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Pointer {
    Moved(Point),
    Down,
    Up,
    /// A press of the secondary button, which on this panel opens the menu on a row
    /// of the Library bay's list and does nothing anywhere else.
    Secondary,
    /// How far to scroll, in logical pixels, positive down the list — a notch of a
    /// mouse wheel converted to `karakuri_console::room::size::WHEEL_STEP` and a
    /// trackpad's own pixels passed straight through.
    Wheel(f32),
}

/// Result of dispatching a pointer event to console controls or engine operations.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Acted {
    /// Nothing acted: a press on a boundary, a move, a wheel, a release.
    Nothing,
    /// The Outputs dot, and what the operation it named did.
    Operated(Outcome),
    /// A fader translated a drag into the vocabulary, or the drag moved the pointer
    /// over a value that did not change and asked for nothing.
    Emitted(Option<Operation>),
    /// Class pill toggle updating environment `Opening` state (ADR-0236).
    Opened,
    /// Moved library cursor row without emitting an engine operation (ADR-0265).
    Pointed,
}
