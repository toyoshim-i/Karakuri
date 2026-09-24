use super::*;

fn try_fake(lines: &str, surface_kind: &str) -> (Result<OutputPlugin, Refusal>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("plugin.sh");
    std::fs::write(&script, lines).expect("write script");
    let command = format!("sh {}", script.display());
    let plugin = OutputPlugin::open(&command, surface_kind, 1920, 1080, "bgra8unorm");
    (plugin, dir)
}

#[test]
fn non_existent_command_fails_gracefully() {
    let outcome = OutputPlugin::open(
        "this_command_definitely_does_not_exist_xyz123",
        "iosurface",
        1920,
        1080,
        "bgra8unorm",
    );
    assert!(matches!(outcome, Err(Refusal::Process(_))));
}

#[test]
fn greeting_with_incompatible_version_is_refused() {
    let script = "echo '{\"t\":\"hello\",\"v\":999,\"kind\":\"output\",\"name\":\"mock\",\"surfaces\":[\"iosurface\"]}'\n";
    let (outcome, _dir) = try_fake(script, "iosurface");
    assert_eq!(
        outcome.err(),
        Some(Refusal::Version {
            theirs: 999,
            ours: PROTOCOL_VERSION,
        })
    );
}

#[test]
fn greeting_with_unsupported_surface_is_refused() {
    let script = "echo '{\"t\":\"hello\",\"v\":1,\"kind\":\"output\",\"name\":\"mock\",\"surfaces\":[\"dxgi\"]}'\n";
    let (outcome, _dir) = try_fake(script, "iosurface");
    assert_eq!(
        outcome.err(),
        Some(Refusal::UnsupportedSurface {
            wanted: "iosurface".to_string(),
            offered: vec!["dxgi".to_string()],
        })
    );
}

#[test]
fn successful_handshake_and_non_blocking_frame_dispatch() {
    let script = concat!(
        "echo '{\"t\":\"hello\",\"v\":1,\"kind\":\"output\",\"name\":\"mock-syphon\",\"surfaces\":[\"iosurface\"]}'\n",
        "read open_line\n",
        "echo '{\"t\":\"ready\",\"server_name\":\"Karakuri-Mock\"}'\n",
        "echo '{\"t\":\"status\",\"clients\":2,\"dropped\":0}'\n",
        "while read line; do\n",
        "  :\n",
        "done\n",
    );
    let (plugin, _dir) = try_fake(script, "iosurface");
    let plugin = plugin.expect("handshake succeeds");

    assert_eq!(plugin.name(), "mock-syphon");
    assert_eq!(plugin.server_name(), "Karakuri-Mock");
    assert!(plugin.is_alive());

    // Allow reader thread brief moment to consume status message
    for _ in 0..50 {
        if plugin.telemetry().clients == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(plugin.telemetry().clients, 2);

    // Dispatching frames must succeed and not block
    for i in 0..10 {
        let sent = plugin.send_frame(i, 42, 1920, 1080);
        assert!(sent || plugin.telemetry().host_dropped > 0);
    }
}

#[test]
fn integration_with_karakuri_syphon_binary() {
    let binary = std::path::Path::new("../../../Karakuri-syphon/target/debug/karakuri-syphon");
    if !binary.exists() {
        return;
    }
    let plugin = OutputPlugin::open(
        binary.to_str().unwrap(),
        "iosurface",
        1920,
        1080,
        "bgra8unorm",
    )
    .expect("karakuri-syphon binary opens successfully");

    assert_eq!(plugin.name(), "syphon");
    assert_eq!(plugin.server_name(), "Karakuri");
    assert!(plugin.is_alive());

    for i in 0..5 {
        let sent = plugin.send_frame(i, 100 + i, 1920, 1080);
        assert!(sent || plugin.telemetry().host_dropped > 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn integration_with_karakuri_spout_binary() {
    let candidates = [
        "../Karakuri-spout/target/debug/karakuri-spout.exe",
        "../../Karakuri-spout/target/debug/karakuri-spout.exe",
        "../../../Karakuri-spout/target/debug/karakuri-spout.exe",
    ];
    let mut command = None;
    for c in candidates {
        let p = std::path::Path::new(c);
        if p.exists() {
            command = Some(c.to_string());
            break;
        }
    }
    let Some(command) = command else {
        return;
    };
    let plugin = OutputPlugin::open(&command, "dxgi", 1920, 1080, "bgra8unorm")
        .expect("karakuri-spout binary opens successfully");

    assert_eq!(plugin.name(), "spout");
    assert_eq!(plugin.server_name(), "Karakuri");
    assert!(plugin.is_alive());

    for i in 0..5 {
        let sent = plugin.send_frame(i, 100 + i, 1920, 1080);
        assert!(sent || plugin.telemetry().host_dropped > 0);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
