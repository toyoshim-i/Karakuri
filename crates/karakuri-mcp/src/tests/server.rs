use super::*;

#[test]
fn serve_at_binds_address_and_port_and_shuts_down() {
    let slots = slots();
    let reporter = serve_at(
        "127.0.0.1:0",
        slots.clone(),
        "a/store".into(),
        true,
        karakuri_environment::Opening::closed(),
        karakuri_environment::SlotPolicies::default(),
    )
    .expect("serve_at binds 127.0.0.1:0");

    assert_eq!(reporter.addr().ip().to_string(), "127.0.0.1");
    assert!(reporter.port() > 0);
    assert_eq!(reporter.port(), reporter.addr().port());

    reporter.shutdown();
}

#[test]
fn serve_at_normalizes_port_only_string() {
    let slots = slots();
    let reporter = serve_at(
        "0",
        slots,
        "a/store".into(),
        true,
        karakuri_environment::Opening::closed(),
        karakuri_environment::SlotPolicies::default(),
    )
    .expect("serve_at binds port 0 as 127.0.0.1:0");

    assert_eq!(reporter.addr().ip().to_string(), "127.0.0.1");
    assert!(reporter.port() > 0);
    drop(reporter);
}
