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
    // 1. Wrapped format
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

    // 2. Direct array format as returned by InProcessMcp::tools()
    let direct_array = json!([
        {
            "name": "read_procedure",
            "description": "Reads a procedure",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "slot": { "type": "integer" }
                }
            }
        }
    ]);
    let direct_openai = mcp_tools_to_openai(&direct_array);
    let d_array = direct_openai.as_array().expect("array of tools");
    assert_eq!(d_array.len(), 1);
    assert_eq!(d_array[0]["function"]["name"], "read_procedure");
}

#[test]
fn test_extract_text_tool_calls_markdown_and_raw_json() {
    use crate::harness::client::extract_text_tool_calls;

    let valid_tools = vec!["operate".to_string(), "read_procedure".to_string()];

    // Markdown ```json block
    let md_input = "Sure, I'll switch decks for you:\n```json\n{\n  \"name\": \"operate\",\n  \"arguments\": {\"operation\": \"Select Deck\", \"with\": {\"deck\": 1}}\n}\n```\nLet me know!";
    let calls = extract_text_tool_calls(md_input, &valid_tools);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "operate");
    assert!(calls[0].1.contains("Select Deck"));

    // <tool_call> tag
    let tag_input = "<tool_call>{\"name\": \"read_procedure\", \"arguments\": {\"slot\": 0, \"layer\": \"L1\"}}</tool_call>";
    let calls = extract_text_tool_calls(tag_input, &valid_tools);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "read_procedure");
    assert!(calls[0].1.contains("\"slot\":0"));

    // Raw JSON object in text
    let raw_input = "Here is the call: {\"name\": \"operate\", \"parameters\": {\"operation\": \"Set BPM\", \"with\": {\"bpm\": 130}}}";
    let calls = extract_text_tool_calls(raw_input, &valid_tools);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "operate");
    assert!(calls[0].1.contains("Set BPM"));
}

#[test]
fn test_normalize_crlf() {
    use crate::harness::normalize_crlf;

    assert_eq!(normalize_crlf("hello\nworld\n"), "hello\r\nworld\r\n");
    assert_eq!(normalize_crlf("hello\r\nworld\r\n"), "hello\r\nworld\r\n");
    assert_eq!(normalize_crlf("a\nb\r\nc\n"), "a\r\nb\r\nc\r\n");
}

#[test]
fn test_prepare_messages_immutable() {
    use crate::harness::client::prepare_messages;

    let msgs = vec![
        json!({
            "role": "system",
            "content": "You are a helpful assistant.",
        }),
        json!({
            "role": "user",
            "content": "Hello",
        }),
    ];

    // Standard mode: immutable, prefix preserved for KV caching
    let regular = prepare_messages(&msgs);
    assert_eq!(regular.len(), 2);
    assert_eq!(regular[0]["role"], "system");
    assert_eq!(regular[1]["role"], "user");
    assert_eq!(regular[1]["content"], "Hello");
}

#[test]
fn test_extract_text_tool_calls_and_clean() {
    use crate::harness::client::extract_text_tool_calls_and_clean;

    let valid = vec!["read_slot".to_string()];

    // Case 1: Pure tool call JSON in markdown block -> cleaned text is None
    let text = "```json\n{\"name\": \"read_slot\", \"arguments\": {\"slot\": 0}}\n```";
    let (calls, clean) = extract_text_tool_calls_and_clean(text, &valid);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].0, "read_slot");
    assert_eq!(clean, None);

    // Case 2: Tool call with reasoning text -> cleaned text preserves reasoning without JSON block
    let text_with_thought = "I will check slot 0 now.\n```json\n{\"name\": \"read_slot\", \"arguments\": {\"slot\": 0}}\n```";
    let (calls2, clean2) = extract_text_tool_calls_and_clean(text_with_thought, &valid);
    assert_eq!(calls2.len(), 1);
    assert_eq!(clean2.as_deref(), Some("I will check slot 0 now."));
}

#[test]
fn test_extract_mcp_result_text() {
    use crate::harness::client::extract_mcp_result_text;

    let rpc_result = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": "{\"bays\": {\"master\": \"off\"}}"
                }
            ]
        }
    });

    let payload = extract_mcp_result_text(&rpc_result);
    assert_eq!(payload, "{\"bays\": {\"master\": \"off\"}}");
}

#[test]
fn test_line_editing_primitives() {
    use crate::harness::editor::{
        insert_str_at, next_word_boundary, prev_word_boundary, remove_char_at, remove_char_range,
    };

    let mut buf = "hello world".to_string();

    // Word boundaries
    assert_eq!(prev_word_boundary(&buf, 11), 6);
    assert_eq!(prev_word_boundary(&buf, 6), 0);
    assert_eq!(next_word_boundary(&buf, 0), 6);
    assert_eq!(next_word_boundary(&buf, 6), 11);

    // Insertion in middle
    insert_str_at(&mut buf, 5, " beautiful");
    assert_eq!(buf, "hello beautiful world");

    // Deletion of single character
    remove_char_at(&mut buf, 5); // remove leading space of " beautiful"
    assert_eq!(buf, "hellobeautiful world");

    // Range deletion (e.g. ⌘Backspace or Ctrl+W)
    remove_char_range(&mut buf, 0, 5);
    assert_eq!(buf, "beautiful world");

    // Multibyte support (Japanese)
    let mut jp = "こんにちは 世界".to_string();
    assert_eq!(prev_word_boundary(&jp, 8), 6);
    assert_eq!(prev_word_boundary(&jp, 6), 0);
    insert_str_at(&mut jp, 5, "、素晴らしい");
    assert_eq!(jp, "こんにちは、素晴らしい 世界");
    remove_char_range(&mut jp, 5, 11);
    assert_eq!(jp, "こんにちは 世界");
}
