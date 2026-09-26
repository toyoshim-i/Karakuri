//! Library bay `.path` row layout, drag-and-drop folder indicators, and read-only behavior
//! (ADR-0275, ADR-0311).

mod common;

use common::{drawn_once, near, rect_of, PLAUSIBLE};
use karakuri_console::input::{claim, Claim};
use karakuri_console::panel::Panel;
use karakuri_console::room::{size, Room};
use karakuri_console::view::{library, LibraryBay, Pointed, Scope, View};
use karakuri_layout::{Point, Rect};

/// The mock's own library, as names, and `library.rs`'s: five Sets in the order
/// it draws them.
fn mock() -> Vec<String> {
    [
        "drift_night",
        "lattice_veil",
        "glass_shell",
        "night01",
        "strand_bloom",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect()
}

/// The four chips a host with a store hands in.
const SCOPES: &[Scope] = &Scope::ALL;

/// The path the mock's own row reads, less the walk: `console.html:112` is
/// `~/sets/tour-2026/night-b › opening`, and what a drop sets is the directory
/// alone — *what the walk inside a folder is* is the one thing ADR-0275
/// explicitly does not decide.
const WHERE: &str = "/Users/somebody/sets/tour-2026/night-b";

/// The other one, for the frames where a second folder is over the window.
const OVER: &str = "/Volumes/stick/handover";

use common::console_panel as console;

/// The bay with the mock's names, pointed at `at` or at nothing.
fn bay(panel: &Panel, at: Option<Pointed<'_>>) -> LibraryBay {
    library(panel.layout(), SCOPES, &mock(), None, at, 0.0).expect("the library bay lists its rows")
}

/// A console with a library in it and a folder chosen — the state a drop leaves
/// behind, built the way the host builds it.
fn pointed_view() -> View {
    let mut view = View::new(Room::Day);
    view.scopes = Scope::ALL.to_vec();
    view.library = mock();
    view.folder = Some(WHERE.to_owned());
    view
}

fn to_egui(r: Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(r.x, r.y), egui::vec2(r.w, r.h))
}

// ---------------------------------------------------------------------------
// Where the row is, and what it costs the rows under it
// ---------------------------------------------------------------------------

/// The row is the mock's own box, under the scopes and over the fields, and
/// every number is read off `style.css` rather than off the panel: `.path {
/// padding: 4px 10px; font-size: 10px; border-bottom: 1px }` is a 24-tall row,
/// its rule the bottom pixel of it.
#[test]
fn the_path_row_is_between_the_scopes_and_the_fields() {
    let panel = console(PLAUSIBLE);
    let region = to_egui(rect_of(panel.layout(), "library"));
    let at = Pointed {
        path: WHERE,
        incoming: false,
    };
    let bay = bay(&panel, Some(at));

    // The transcription itself, against the stylesheet's own numbers.
    assert!(near(size::PATH_PAD_X, 10.0));
    assert!(near(size::PATH_PAD_Y, 4.0));
    assert!(near(size::PATH_SIZE, 10.0));
    assert!(
        near(size::PATH_H, 24.0),
        "`.path` is 4 + 10 x 1.5 + 4 and a rule, which is 24 and not {}",
        size::PATH_H
    );

    let scopes = bay.scopes.expect("the bay was handed scopes");
    let path = bay.path.expect("the bay was pointed at a folder");
    let filters = bay.filters.expect("the bay is wide enough for two fields");

    assert!(
        near(path.min.y, scopes.max.y),
        "the path row starts at {} and the scope row ends at {}",
        path.min.y,
        scopes.max.y
    );
    assert!(near(path.height(), size::PATH_H));
    assert!(
        near(path.min.x, region.min.x) && near(path.max.x, region.max.x),
        "the row is the full width of the bay in the mock and is {path:?} here"
    );
    // The horizontal rule is drawn inside the path row bounds, aligning filters directly below.
    assert!(
        near(filters.min.y, path.max.y),
        "the filter row starts at {} where the path row ends at {}",
        filters.min.y,
        path.max.y
    );
}

/// Without a selected folder the path row is omitted; when present, subsequent items shift down by its height.
#[test]
fn a_bay_pointed_nowhere_is_the_bay_it_was_and_one_pointed_costs_exactly_the_row() {
    let panel = console(PLAUSIBLE);
    let bare = bay(&panel, None);
    let at = Pointed {
        path: WHERE,
        incoming: false,
    };
    let shown = bay(&panel, Some(at));

    assert_eq!(bare.path, None, "a bay nobody pointed drew a `.path` row");
    let bare_scopes = bare.scopes.expect("the bay was handed scopes");
    let bare_filters = bare.filters.expect("two fields fit");
    assert!(
        near(bare_filters.min.y, bare_scopes.max.y),
        "with no path row the fields do not sit on the scopes"
    );

    assert!(near(shown.scopes.unwrap().min.y, bare_scopes.min.y));
    assert!(
        near(
            shown.filters.unwrap().min.y,
            bare_filters.min.y + size::PATH_H
        ),
        "the filter row moved by something other than the path row's own height"
    );
    assert!(
        near(shown.list.min.y, bare.list.min.y + size::PATH_H),
        "the list moved by something other than the path row's own height"
    );
    // The foot is on the bottom edge and does not move: the leftover is the
    // list's, so what the row costs is rows.
    assert!(near(shown.foot.min.y, bare.foot.min.y));
    assert!(
        shown.rows <= bare.rows,
        "the bay grew rows by drawing another row of furniture"
    );
    assert!(
        bare.rows - shown.rows <= 1,
        "a 24-tall row cost {} rows of 22.5",
        bare.rows - shown.rows
    );
}

/// The row is not the scope row's condition. A console handed no chips at all
/// still says where it is pointed, because that row is also where a send's save
/// dialog opens (ADR-0311) — so it is drawn straight under the bay head, where
/// the fields and the chips are not drawn at all.
#[test]
fn a_console_with_no_chips_still_says_where_it_is_pointed() {
    let panel = console(PLAUSIBLE);
    let region = to_egui(rect_of(panel.layout(), "library"));
    let at = Pointed {
        path: WHERE,
        incoming: false,
    };
    let bay = library(panel.layout(), &[], &mock(), None, Some(at), 0.0)
        .expect("a bay with rows in it and no chips over them");

    assert_eq!(
        bay.scopes, None,
        "a console handed no scopes drew a chip row"
    );
    assert_eq!(
        bay.filters, None,
        "the fields are the scope row's condition"
    );
    let path = bay.path.expect("the bay was pointed at a folder");
    assert!(
        near(path.min.y, region.min.y + size::HEAD_H),
        "the path row is at {} and the bay head ends at {}",
        path.min.y,
        region.min.y + size::HEAD_H
    );
    assert!(near(bay.list.min.y, path.max.y + size::LIB_LIST_PAD));
}

// ---------------------------------------------------------------------------
// What the row reads, and in which ink
// ---------------------------------------------------------------------------

/// A hovered folder path supersedes the chosen folder, while multiple hovered paths are rejected.
#[test]
fn a_folder_over_the_window_replaces_the_line_and_a_bay_pointed_nowhere_has_none() {
    let mut view = View::new(Room::Day);
    assert_eq!(
        view.pointed(),
        None,
        "a console nobody has pointed anywhere draws a `.path` row"
    );

    view.folder = Some(WHERE.to_owned());
    assert_eq!(
        view.pointed(),
        Some(Pointed {
            path: WHERE,
            incoming: false,
        })
    );

    view.incoming = Some(OVER.to_owned());
    assert_eq!(
        view.pointed(),
        Some(Pointed {
            path: OVER,
            incoming: true,
        }),
        "the row reads the folder that was chosen while another is over the window"
    );

    // Out of the window again, and the row goes back to what it said.
    view.incoming = None;
    assert_eq!(
        view.pointed(),
        Some(Pointed {
            path: WHERE,
            incoming: false,
        })
    );

    // A folder over a bay that was pointed nowhere: the row appears for the
    // length of the drag, which is the announcement.
    view.folder = None;
    view.incoming = Some(OVER.to_owned());
    assert_eq!(
        view.pointed(),
        Some(Pointed {
            path: OVER,
            incoming: true,
        })
    );
}

/// Every shape the console paints inside `rect`, on one frame — `library.rs`'s
/// own helper.
fn shapes_inside(view: &mut View, panel: &mut Panel, rect: egui::Rect) -> Vec<egui::Shape> {
    let ctx = drawn_once();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| view.draw(ui, panel));
    out.textures_delta.clear();
    out.shapes
        .into_iter()
        .filter(|clipped| {
            let bounds = clipped.shape.visual_bounding_rect();
            bounds.is_finite() && rect.contains_rect(bounds)
        })
        .map(|clipped| clipped.shape)
        .collect()
}

/// The one line of type the path row painted: the words, and the ink.
fn line(view: &mut View, panel: &mut Panel, row: egui::Rect) -> (String, egui::Color32) {
    let texts: Vec<(String, egui::Color32)> = shapes_inside(view, panel, row)
        .iter()
        .filter_map(|shape| match shape {
            egui::Shape::Text(at) => Some((at.galley.text().to_owned(), at.fallback_color)),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts.len(),
        1,
        "the path row painted {} lines of type and the mock draws one",
        texts.len()
    );
    texts.into_iter().next().expect("the one line")
}

/// The path is painted, and the ink is the mark. `--c-faint` for the row as it
/// stands and `--c-text` while a folder is over the window, which is `.path`
/// against `.path.incoming` and is the whole of what this gesture can afford to
/// draw.
#[test]
fn the_row_reads_the_path_and_comes_up_out_of_its_faint_while_one_is_over_the_window() {
    let mut panel = console(PLAUSIBLE);
    let mut view = pointed_view();
    let pal = Room::Day.palette();
    let row = bay(
        &panel,
        Some(Pointed {
            path: WHERE,
            incoming: false,
        }),
    )
    .path
    .expect("the bay was pointed at a folder");

    let (text, ink) = line(&mut view, &mut panel, row);
    assert_eq!(text, WHERE, "the row reads `{text}`");
    assert_eq!(
        ink, pal.faint,
        "`.path` is `--c-faint` until one is incoming"
    );

    view.incoming = Some(OVER.to_owned());
    let (text, ink) = line(&mut view, &mut panel, row);
    assert_eq!(
        text, OVER,
        "the row reads `{text}` while a folder is over the window"
    );
    assert_eq!(ink, pal.text, "`.path.incoming` is `--c-text`");
}

// ---------------------------------------------------------------------------
// It is a readout
// ---------------------------------------------------------------------------

/// The path row is strictly a readout; presses pass through to egui (ADR-0305).
#[test]
fn the_path_row_answers_no_press() {
    let mut panel = console(PLAUSIBLE);
    let view = pointed_view();
    let ctx = drawn_once();
    let row = bay(
        &panel,
        Some(Pointed {
            path: WHERE,
            incoming: false,
        }),
    )
    .path
    .expect("the bay was pointed at a folder");

    for probe in [
        egui::pos2(row.min.x + size::PATH_PAD_X, row.center().y),
        row.center(),
        egui::pos2(row.max.x - size::PATH_PAD_X, row.center().y),
    ] {
        assert_eq!(
            claim(&mut panel, &ctx, &view, Point::new(probe.x, probe.y)),
            Claim::Egui,
            "the console took the pointer at {probe:?}, which is on the `.path` row"
        );
    }
}
