//! Unit tests for in-process harness components (menu, config, and tool conversion).

use crate::harness::client::mcp_tools_to_openai;
use crate::harness::config::HarnessConfig;
use crate::harness::menu::{MenuAction, ModelMenu};
use crate::harness::probe::DiscoveredModel;
use serde_json::json;

#[test]
fn test_menu_navigation_and_selection() {
    let models = vec![
        DiscoveredModel {
            server_name: "Ollama".into(),
            base_url: "http://localhost:11434".into(),
            model_id: "llama3.2:latest".into(),
        },
        DiscoveredModel {
            server_name: "Ollama".into(),
            base_url: "http://localhost:11434".into(),
            model_id: "qwen2.5:7b".into(),
        },
        DiscoveredModel {
            server_name: "LM Studio".into(),
            base_url: "http://localhost:1234".into(),
            model_id: "deepseek-r1".into(),
        },
    ];

    let config = HarnessConfig::default();
    let mut menu = ModelMenu::new(models.clone(), &config);
    assert_eq!(menu.selected_index, 0);

    // Move down
    assert!(matches!(menu.handle_key(b"\x1b[B"), MenuAction::Redraw));
    assert_eq!(menu.selected_index, 1);

    // Move down
    assert!(matches!(menu.handle_key(b"\x1b[B"), MenuAction::Redraw));
    assert_eq!(menu.selected_index, 2);

    // Wrap around down -> 0
    assert!(matches!(menu.handle_key(b"\x1b[B"), MenuAction::Redraw));
    assert_eq!(menu.selected_index, 0);

    // Wrap around up -> 2
    assert!(matches!(menu.handle_key(b"\x1b[A"), MenuAction::Redraw));
    assert_eq!(menu.selected_index, 2);

    // Direct number selection '2' -> index 1
    match menu.handle_key(b"2") {
        MenuAction::Selected(m) => {
            assert_eq!(m.model_id, "qwen2.5:7b");
        }
        _ => panic!("expected MenuAction::Selected"),
    }

    // Enter selection
    match menu.handle_key(b"\r") {
        MenuAction::Selected(m) => {
            assert_eq!(m.model_id, "qwen2.5:7b");
        }
        _ => panic!("expected MenuAction::Selected"),
    }

    // Cancellation
    assert!(matches!(menu.handle_key(b"\x1b"), MenuAction::Cancelled));
    assert!(matches!(menu.handle_key(b"\x03"), MenuAction::Cancelled));
}

#[test]
fn test_mcp_tools_to_openai_schema() {
    let mcp_schema = json!({
        "tools": [
            {
                "name": "operate",
                "description": "Performs an operation on the mixer deck.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "op": { "type": "string" }
                    },
                    "required": ["op"]
                }
            }
        ]
    });

    let openai_tools = mcp_tools_to_openai(&mcp_schema);
    let array = openai_tools.as_array().expect("array of tools");
    assert_eq!(array.len(), 1);

    let tool = &array[0];
    assert_eq!(tool["type"], "function");
    assert_eq!(tool["function"]["name"], "operate");
    assert_eq!(
        tool["function"]["description"],
        "Performs an operation on the mixer deck."
    );
    assert_eq!(
        tool["function"]["parameters"]["properties"]["op"]["type"],
        "string"
    );
}
