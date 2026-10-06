//! MCP tool inspection and CLI command execution helpers for Prompt Bay.

use serde_json::{json, Value};

/// Formats the general usage help text for `/call`.
pub fn call_usage_help() -> String {
    let mut out = String::new();
    out.push_str(
        "\x1b[1;36mUsage:\x1b[0m /call \x1b[1;32m<tool_name>\x1b[0m \x1b[33m[args]\x1b[0m\r\n\r\n",
    );
    out.push_str("\x1b[1mArgument formats supported:\x1b[0m\r\n");
    out.push_str("  1. Positional:  /call read_procedure 0 L1\r\n");
    out.push_str("  2. Key=Value:   /call read_procedure slot=0 layer=L1\r\n");
    out.push_str("  3. JSON Object: /call read_procedure {\"slot\": 0, \"layer\": \"L1\"}\r\n\r\n");
    out.push_str("\x1b[1mExamples:\x1b[0m\r\n");
    out.push_str("  /call list_sets\r\n");
    out.push_str("  /call read_resource karakuri://operations\r\n");
    out.push_str(
        "  /call operate {\"operation\": \"Gain\", \"with\": {\"deck\": 0, \"gain\": 0.8}}\r\n\r\n",
    );
    out.push_str("Type \x1b[1;32m/tools\x1b[0m to list all registered tools.\r\n");
    out
}

/// Retrieves the slice or vector of tool objects from the MCP tools value.
pub fn get_tools_array(mcp_tools: &Value) -> Vec<&Value> {
    if let Some(arr) = mcp_tools.as_array() {
        arr.iter().collect()
    } else if let Some(arr) = mcp_tools.get("tools").and_then(|t| t.as_array()) {
        arr.iter().collect()
    } else {
        Vec::new()
    }
}

/// Retrieves all registered tool names.
pub fn get_tool_names(mcp_tools: &Value) -> Vec<String> {
    get_tools_array(mcp_tools)
        .iter()
        .filter_map(|t| {
            t.get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        })
        .collect()
}

/// Finds a specific tool schema definition by name.
pub fn find_tool_schema<'a>(mcp_tools: &'a Value, name: &str) -> Option<&'a Value> {
    get_tools_array(mcp_tools)
        .into_iter()
        .find(|t| t.get("name").and_then(|n| n.as_str()) == Some(name))
}

/// Builds parameter usage hint string for a tool, e.g. `<slot> <layer> [index]`.
pub fn build_param_hint(tool: &Value) -> String {
    let schema = match tool.get("inputSchema") {
        Some(s) => s,
        None => return String::new(),
    };

    let props = match schema.get("properties").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return String::new(),
    };

    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut hints = Vec::new();
    for req in &required {
        if props.contains_key(*req) {
            hints.push(format!("<{req}>"));
        }
    }
    for key in props.keys() {
        if !required.contains(&key.as_str()) {
            hints.push(format!("[{key}]"));
        }
    }

    hints.join(" ")
}

/// Formats the overview list of all available WebMCP tools.
pub fn format_tools_overview(mcp_tools: &Value) -> String {
    let tools = get_tools_array(mcp_tools);
    let mut out = String::new();
    out.push_str(&format!(
        "\x1b[1;36mAvailable MCP Tools ({})\x1b[0m\r\n",
        tools.len()
    ));

    for tool in tools {
        let name = tool.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let hint = build_param_hint(tool);
        let desc = tool
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("");

        let short_desc = extract_short_desc(desc);

        if hint.is_empty() {
            out.push_str(&format!("  \x1b[1;32m{:<18}\x1b[0m\r\n", name));
        } else {
            out.push_str(&format!(
                "  \x1b[1;32m{:<18}\x1b[0m \x1b[33m{}\x1b[0m\r\n",
                name, hint
            ));
        }
        if !short_desc.is_empty() {
            out.push_str(&format!("    \x1b[2m{}\x1b[0m\r\n", short_desc));
        }
    }

    out.push_str(
        "\r\n\x1b[2mUse '/tools <name>' for details, '/call <name> [args]' to execute.\x1b[0m\r\n",
    );
    out
}

fn extract_short_desc(desc: &str) -> String {
    let trimmed = desc.trim();
    if let Some(pos) = trimmed.find(". ") {
        trimmed[..=pos].to_string()
    } else if let Some(pos) = trimmed.find(".\n") {
        trimmed[..=pos].to_string()
    } else if let Some(pos) = trimmed.find('\n') {
        trimmed[..pos].to_string()
    } else if trimmed.chars().count() > 80 {
        let mut s: String = trimmed.chars().take(77).collect();
        s.push_str("...");
        s
    } else {
        trimmed.to_string()
    }
}

/// Formats detailed schema and usage information for a specific tool.
pub fn format_tool_detail(mcp_tools: &Value, name: &str) -> String {
    let tool = match find_tool_schema(mcp_tools, name) {
        Some(t) => t,
        None => {
            return format!(
                "\x1b[31mTool '{name}' not found.\x1b[0m Type \x1b[1;32m/tools\x1b[0m to list all tools.\r\n"
            );
        }
    };

    let mut out = String::new();
    out.push_str(&format!(
        "\x1b[1;36mMCP Tool:\x1b[0m \x1b[1;32m{name}\x1b[0m\r\n\r\n"
    ));

    let desc = tool
        .get("description")
        .and_then(|d| d.as_str())
        .unwrap_or("(No description)");
    out.push_str("\x1b[1mDescription:\x1b[0m\r\n");
    for line in desc.lines() {
        out.push_str(&format!("  {line}\r\n"));
    }
    out.push_str("\r\n");

    let schema = tool.get("inputSchema");
    let props = schema
        .and_then(|s| s.get("properties"))
        .and_then(|p| p.as_object());
    let required: Vec<&str> = schema
        .and_then(|s| s.get("required"))
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    out.push_str("\x1b[1mParameters:\x1b[0m\r\n");
    if let Some(props) = props {
        if props.is_empty() {
            out.push_str("  (None)\r\n");
        } else {
            for (prop_name, prop_def) in props {
                let is_req = required.contains(&prop_name.as_str());
                let p_type = prop_def
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("any");
                let status = if is_req {
                    "\x1b[1;31mrequired\x1b[0m"
                } else {
                    "\x1b[2moptional\x1b[0m"
                };

                out.push_str(&format!(
                    "  • \x1b[1;33m{prop_name}\x1b[0m ({p_type}, {status})\r\n"
                ));

                if let Some(p_desc) = prop_def.get("description").and_then(|d| d.as_str()) {
                    for line in p_desc.lines() {
                        out.push_str(&format!("      \x1b[2m{line}\x1b[0m\r\n"));
                    }
                }
                if let Some(enums) = prop_def.get("enum").and_then(|e| e.as_array()) {
                    let enum_str: Vec<String> = enums
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| v.to_string())
                        })
                        .collect();
                    out.push_str(&format!(
                        "      \x1b[35mValues:\x1b[0m [{}]\r\n",
                        enum_str.join(", ")
                    ));
                }
            }
        }
    } else {
        out.push_str("  (None)\r\n");
    }

    out.push_str("\r\n\x1b[1mExamples:\x1b[0m\r\n");
    if let Some(props) = props {
        if props.is_empty() {
            out.push_str(&format!("  /call {name}\r\n"));
        } else {
            let mut kv_parts = Vec::new();
            let mut pos_parts = Vec::new();
            let mut json_map = serde_json::Map::new();

            for req in &required {
                if let Some(p_def) = props.get(*req) {
                    let example_val = generate_example_val(p_def);
                    kv_parts.push(format!("{req}={example_val}"));
                    pos_parts.push(example_val.clone());
                    json_map.insert((*req).to_string(), parse_simple_val(&example_val, p_def));
                }
            }

            if !kv_parts.is_empty() {
                out.push_str(&format!("  /call {name} {}\r\n", kv_parts.join(" ")));
                out.push_str(&format!("  /call {name} {}\r\n", pos_parts.join(" ")));
                out.push_str(&format!("  /call {name} {}\r\n", Value::Object(json_map)));
            } else {
                out.push_str(&format!("  /call {name}\r\n"));
            }
        }
    } else {
        out.push_str(&format!("  /call {name}\r\n"));
    }

    out
}

fn generate_example_val(prop_def: &Value) -> String {
    if let Some(enums) = prop_def.get("enum").and_then(|e| e.as_array()) {
        if let Some(first) = enums.first().and_then(|v| v.as_str()) {
            return first.to_string();
        }
    }
    match prop_def.get("type").and_then(|t| t.as_str()) {
        Some("integer") => "0".to_string(),
        Some("number") => "1.0".to_string(),
        Some("boolean") => "true".to_string(),
        _ => "value".to_string(),
    }
}

fn parse_simple_val(val_str: &str, prop_def: &Value) -> Value {
    match prop_def.get("type").and_then(|t| t.as_str()) {
        Some("integer") => val_str
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or(Value::String(val_str.to_string())),
        Some("number") => val_str
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or(Value::String(val_str.to_string())),
        Some("boolean") => val_str
            .parse::<bool>()
            .map(Value::from)
            .unwrap_or(Value::String(val_str.to_string())),
        _ => Value::String(val_str.to_string()),
    }
}

/// Parses CLI argument strings (JSON, key=value, or positional) into a JSON argument object.
pub fn parse_call_args(args_str: &str, tool_schema: Option<&Value>) -> Result<Value, String> {
    let trimmed = args_str.trim();
    let parsed_value = if trimmed.is_empty() {
        json!({})
    } else if trimmed.starts_with('{') {
        let v: Value =
            serde_json::from_str(trimmed).map_err(|e| format!("Invalid JSON arguments: {e}"))?;
        if !v.is_object() {
            return Err("JSON arguments must be an object, e.g. {\"key\": \"value\"}".into());
        }
        v
    } else if trimmed.contains('=') {
        let pairs = parse_kv_tokens(trimmed)?;
        let mut map = serde_json::Map::new();
        for (k, v_str) in pairs {
            let val = convert_token_value(&k, &v_str, tool_schema);
            map.insert(k, val);
        }
        Value::Object(map)
    } else {
        // Positional arguments
        let tokens = tokenize_positional(trimmed);
        if let Some(schema) = tool_schema {
            let props_order = get_schema_properties_order(schema);
            if tokens.len() > props_order.len() {
                return Err(format!(
                    "Too many arguments: expected at most {}, got {}",
                    props_order.len(),
                    tokens.len()
                ));
            }
            let mut map = serde_json::Map::new();
            for (i, token) in tokens.iter().enumerate() {
                let prop_name = &props_order[i];
                let val = convert_token_value(prop_name, token, tool_schema);
                map.insert(prop_name.clone(), val);
            }
            Value::Object(map)
        } else {
            return Err("Unknown tool schema; please use JSON: {\"key\": \"value\"}".into());
        }
    };

    // Verify required properties if schema is available
    if let Some(schema) = tool_schema {
        if let Some(req_arr) = schema
            .get("inputSchema")
            .and_then(|s| s.get("required"))
            .and_then(|r| r.as_array())
        {
            if let Some(map) = parsed_value.as_object() {
                for req in req_arr {
                    if let Some(key) = req.as_str() {
                        if !map.contains_key(key) {
                            let tool_name =
                                schema.get("name").and_then(|n| n.as_str()).unwrap_or("");
                            let hint = build_param_hint(schema);
                            return Err(format!(
                                "Missing required parameter '{key}'. Usage: /call {tool_name} {hint}"
                            ));
                        }
                    }
                }
            }
        }
    }

    Ok(parsed_value)
}

fn get_schema_properties_order(schema: &Value) -> Vec<String> {
    let input_schema = match schema.get("inputSchema") {
        Some(s) => s,
        None => return Vec::new(),
    };

    let props = match input_schema.get("properties").and_then(|p| p.as_object()) {
        Some(p) => p,
        None => return Vec::new(),
    };

    let required: Vec<String> = input_schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let mut order = Vec::new();
    for req in &required {
        if props.contains_key(req) {
            order.push(req.clone());
        }
    }
    for key in props.keys() {
        if !order.contains(key) {
            order.push(key.clone());
        }
    }

    order
}

fn parse_kv_tokens(input: &str) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        let key_start = i;
        while i < len && chars[i] != '=' && !chars[i].is_whitespace() {
            i += 1;
        }
        let key: String = chars[key_start..i].iter().collect();

        if i >= len || chars[i] != '=' {
            return Err(format!("Expected '=' after key '{key}'"));
        }
        i += 1; // skip '='

        if i >= len {
            pairs.push((key, String::new()));
            break;
        }

        let mut val = String::new();
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < len {
                    val.push(chars[i + 1]);
                    i += 2;
                } else {
                    val.push(chars[i]);
                    i += 1;
                }
            }
            if i < len && chars[i] == quote {
                i += 1;
            }
        } else if chars[i] == '{' || chars[i] == '[' {
            let open_ch = chars[i];
            let close_ch = if open_ch == '{' { '}' } else { ']' };
            let mut depth = 0;
            while i < len {
                val.push(chars[i]);
                if chars[i] == open_ch {
                    depth += 1;
                } else if chars[i] == close_ch {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
        } else {
            while i < len && !chars[i].is_whitespace() {
                val.push(chars[i]);
                i += 1;
            }
        }

        pairs.push((key, val));
    }

    Ok(pairs)
}

fn tokenize_positional(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        while i < len && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= len {
            break;
        }

        let mut token = String::new();
        if chars[i] == '"' || chars[i] == '\'' {
            let quote = chars[i];
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < len {
                    token.push(chars[i + 1]);
                    i += 2;
                } else {
                    token.push(chars[i]);
                    i += 1;
                }
            }
            if i < len && chars[i] == quote {
                i += 1;
            }
        } else if chars[i] == '{' || chars[i] == '[' {
            let open_ch = chars[i];
            let close_ch = if open_ch == '{' { '}' } else { ']' };
            let mut depth = 0;
            while i < len {
                token.push(chars[i]);
                if chars[i] == open_ch {
                    depth += 1;
                } else if chars[i] == close_ch {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                i += 1;
            }
        } else {
            while i < len && !chars[i].is_whitespace() {
                token.push(chars[i]);
                i += 1;
            }
        }
        tokens.push(token);
    }

    tokens
}

fn convert_token_value(key: &str, val_str: &str, tool_schema: Option<&Value>) -> Value {
    let expected_type = tool_schema.and_then(|s| {
        s.get("inputSchema")
            .and_then(|is| is.get("properties"))
            .and_then(|p| p.get(key))
            .and_then(|pdef| pdef.get("type"))
            .and_then(|t| t.as_str())
    });

    if val_str == "true" {
        return Value::Bool(true);
    }
    if val_str == "false" {
        return Value::Bool(false);
    }
    if val_str == "null" {
        return Value::Null;
    }

    if (val_str.starts_with('{') && val_str.ends_with('}'))
        || (val_str.starts_with('[') && val_str.ends_with(']'))
    {
        if let Ok(json_val) = serde_json::from_str::<Value>(val_str) {
            return json_val;
        }
    }

    match expected_type {
        Some("integer") => {
            if let Ok(n) = val_str.parse::<i64>() {
                return json!(n);
            }
        }
        Some("number") => {
            if let Ok(n) = val_str.parse::<f64>() {
                return json!(n);
            }
        }
        Some("boolean") => {
            if let Ok(b) = val_str.parse::<bool>() {
                return json!(b);
            }
        }
        Some("string") => {
            return Value::String(val_str.to_string());
        }
        _ => {}
    }

    if let Ok(n) = val_str.parse::<i64>() {
        json!(n)
    } else if let Ok(f) = val_str.parse::<f64>() {
        json!(f)
    } else {
        Value::String(val_str.to_string())
    }
}

/// Formats the tool execution result or error for terminal display.
pub fn format_call_result(tool_name: &str, res: Result<Value, String>) -> String {
    match res {
        Ok(val) => {
            if let Some(err) = val.get("error") {
                if let Some(msg) = err.get("message").and_then(|m| m.as_str()) {
                    return format!("\x1b[31m✖ Error from {tool_name}:\x1b[0m\r\n{msg}");
                }
            }

            let is_error = val
                .get("isError")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                || val
                    .get("result")
                    .and_then(|r| r.get("isError"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                || val.get("error").is_some();

            let text = crate::harness::client::extract_mcp_result_text(&val);

            if is_error {
                format!("\x1b[31m✖ Error from {tool_name}:\x1b[0m\r\n{text}")
            } else {
                let display_text = if let Ok(parsed_json) = serde_json::from_str::<Value>(&text) {
                    if parsed_json.is_object() || parsed_json.is_array() {
                        serde_json::to_string_pretty(&parsed_json).unwrap_or(text)
                    } else {
                        text
                    }
                } else {
                    text
                };

                format!("\x1b[1;32m✔ Result:\x1b[0m\r\n{display_text}")
            }
        }
        Err(err) => {
            format!("\x1b[31m✖ Failed to execute {tool_name}: {err}\x1b[0m")
        }
    }
}
