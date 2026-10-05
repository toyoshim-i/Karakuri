//! Tests for the transport row MCP server configuration pill and dropdown card.

mod common;

use common::mock_transport;
use karakuri_console::room::Room;
use karakuri_console::view::{
    mcp_server_pill, McpAsk, McpServer, ModalOverlay, TextInputKind, View, DEFAULT_MCP_ADDR,
    MCP_MENU_ROWS, PRESET_ADDRS,
};
use karakuri_layout::Point;

#[test]
fn mcp_server_defaults_and_lifecycle() {
    let mut mcp = McpServer::new();
    assert_eq!(mcp.addr, DEFAULT_MCP_ADDR);
    assert!(!mcp.running);
    assert!(!mcp.open());
    assert_eq!(mcp.rows(), 0);

    mcp.opened();
    assert!(mcp.open());
    assert_eq!(mcp.rows(), MCP_MENU_ROWS);

    mcp.shut();
    assert!(!mcp.open());
    assert_eq!(mcp.rows(), 0);
}

#[test]
fn mcp_server_inline_editing() {
    let mut mcp = McpServer::new();
    assert_eq!(mcp.editing(), None);

    mcp.start_editing();
    assert_eq!(mcp.editing(), Some(DEFAULT_MCP_ADDR));

    assert!(mcp.typed(':'));
    assert_eq!(mcp.editing(), Some("127.0.0.1:4040:"));

    assert!(mcp.rubbed_out());
    assert_eq!(mcp.editing(), Some(DEFAULT_MCP_ADDR));

    mcp.shut();
    assert_eq!(mcp.editing(), None);
}

#[test]
fn mcp_server_modal_overlay_and_text_input() {
    let mut view = View::new(Room::Day);
    assert_eq!(view.active_overlay(), None);
    assert_eq!(view.active_text_input(), None);

    view.mcp_server.opened();
    assert_eq!(view.active_overlay(), Some(ModalOverlay::McpServerMenu));
    assert!(view.has_modal_overlay());
    assert_eq!(view.active_text_input(), None);

    view.mcp_server.start_editing();
    assert_eq!(view.active_overlay(), Some(ModalOverlay::McpServerMenu));
    assert_eq!(view.active_text_input(), Some(TextInputKind::McpServerAddr));

    assert!(view.type_into_active("9"));
    assert_eq!(view.mcp_server.editing(), Some("127.0.0.1:40409"));

    assert!(view.rub_out_active());
    assert_eq!(view.mcp_server.editing(), Some(DEFAULT_MCP_ADDR));

    assert!(view.cancel_active_text_input());
    assert_eq!(view.active_overlay(), None);
    assert_eq!(view.active_text_input(), None);
}

#[test]
fn mcp_server_pill_geometry_and_placement() {
    let (panel, ctx) = common::console(common::PLAUSIBLE);
    let transport = Some(mock_transport());
    let view = View::new(Room::Day);

    let row = karakuri_console::view::transport(&ctx, panel.layout(), transport).expect("a row");
    let pill = mcp_server_pill(
        &ctx,
        panel.layout(),
        transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        &view.mcp_server,
        view.theme_mode,
        false,
    )
    .expect("mcp server pill to be placed");

    assert!(pill.pill.max.x < row.frame.min.x);
    assert!(pill.pill.min.x > row.bar.max.x);
}

#[test]
fn mcp_server_pill_hit_testing_and_dropdown() {
    let (panel, ctx) = common::console(common::PLAUSIBLE);
    let transport = Some(mock_transport());
    let mut view = View::new(Room::Day);

    let pill_shut = mcp_server_pill(
        &ctx,
        panel.layout(),
        transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        &view.mcp_server,
        view.theme_mode,
        false,
    )
    .expect("mcp pill shut");

    let pill_center = Point::new(pill_shut.pill.center().x, pill_shut.pill.center().y);
    assert_eq!(
        pill_shut.ask(&view.mcp_server, pill_center),
        Some(McpAsk::ToggleMenu)
    );

    let outside = Point::new(pill_shut.pill.min.x - 20.0, pill_shut.pill.min.y);
    assert_eq!(pill_shut.ask(&view.mcp_server, outside), None);

    view.mcp_server.opened();
    let pill_open = mcp_server_pill(
        &ctx,
        panel.layout(),
        transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        &view.mcp_server,
        view.theme_mode,
        false,
    )
    .expect("mcp pill open");

    assert!(pill_open.menu.is_some());
    assert_eq!(pill_open.rows, MCP_MENU_ROWS);

    // Row 0: Toggle running
    let row0 = pill_open.row(0).expect("row 0");
    let p0 = Point::new(row0.center().x, row0.center().y);
    assert_eq!(
        pill_open.ask(&view.mcp_server, p0),
        Some(McpAsk::ToggleRunning)
    );

    // Row 1: Edit address
    let row1 = pill_open.row(1).expect("row 1");
    let p1 = Point::new(row1.center().x, row1.center().y);
    assert_eq!(pill_open.ask(&view.mcp_server, p1), Some(McpAsk::EditAddr));

    // Rows 2..4: Presets
    for (i, &preset) in PRESET_ADDRS.iter().enumerate() {
        let row = pill_open.row(2 + i).expect("preset row");
        let p = Point::new(row.center().x, row.center().y);
        assert_eq!(
            pill_open.ask(&view.mcp_server, p),
            Some(McpAsk::SetAddr(preset.to_string()))
        );
    }
}

#[test]
fn webmcp_server_pill_behavior() {
    let mut mcp = McpServer::web();
    assert_eq!(mcp.addr, "WebMCP");
    assert!(mcp.running);
    assert!(mcp.is_webmcp);
    assert!(!mcp.open());
    assert_eq!(mcp.rows(), 0);

    mcp.opened();
    assert!(mcp.open());
    assert_eq!(mcp.rows(), karakuri_console::view::WEBMCP_MENU_ROWS);

    let (panel, ctx) = common::console(common::PLAUSIBLE);
    let transport = Some(mock_transport());
    let mut view = View::new(Room::Day);
    view.mcp_server = mcp;

    let pill = mcp_server_pill(
        &ctx,
        panel.layout(),
        transport,
        view.audio.as_ref(),
        view.tracker,
        view.map.as_ref(),
        &view.arrangement,
        &view.mcp_server,
        view.theme_mode,
        false,
    )
    .expect("webmcp pill open");

    assert_eq!(pill.rows, karakuri_console::view::WEBMCP_MENU_ROWS);

    // Clicking anywhere in open webmcp card dismisses/shuts without toggling TCP server
    let row0 = pill.row(0).expect("row 0");
    let p0 = Point::new(row0.center().x, row0.center().y);
    assert_eq!(pill.ask(&view.mcp_server, p0), Some(McpAsk::Shut));
}
