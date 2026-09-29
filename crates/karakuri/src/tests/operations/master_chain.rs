use super::*;
use karakuri_console::view::{master, AddChoices};
use karakuri_environment::mix::{build_chain, resolve_procedure, shipped};

#[test]
fn adding_l5_to_master_chain_populates_view_params_and_allows_interaction_and_removal() {
    let gpu = pollster::block_on(Gpu::new(None)).expect("headless gpu");
    let mut present =
        karakuri_engine::Present::new(&gpu.device, karakuri_engine::Present::HDR_FORMAT, 1280, 720);

    // 1. Resolve L5 procedure (zoom_blur)
    let zoom_blur_src = shipped::ZOOM_BLUR;
    let proc_address = shipped::address(zoom_blur_src);
    assert_eq!(
        resolve_procedure(None, &proc_address).as_deref(),
        Some(zoom_blur_src)
    );

    // 2. Build and install chain with zoom_blur into present
    let slot_spec = karakuri_engine::SlotSpec {
        procedure: proc_address.clone(),
        cut: None,
        params: std::collections::BTreeMap::new(),
    };
    let chain = build_chain(&gpu.device, present.chain_layout(), &[slot_spec], &|addr| {
        resolve_procedure(None, addr)
    })
    .expect("zoom_blur compiles into master chain");

    drop(present.set_chain(&gpu.device, &gpu.queue, chain));

    // 3. Verify present chain_reading reflects the installed slot and declared parameters
    let reading = present.chain_reading();
    assert_eq!(reading.len(), 1);
    assert_eq!(reading[0].procedure, proc_address);
    assert_eq!(reading[0].name, "zoom_blur");
    assert!(
        reading[0].params.len() >= 3,
        "zoom_blur declares amount, kick, snap, origin.x, origin.y"
    );

    // 4. Construct chain_view and verify console layout
    let view_chain = crate::bridge::chain_view(&present, &[]);
    assert_eq!(view_chain.slots.len(), 1);
    assert_eq!(view_chain.slots[0].name, "zoom blur");
    assert_eq!(view_chain.slots[0].params.len(), reading[0].params.len());

    let ctx = crate::tests::drawn_once();
    let mut readout = crate::readout::Readout::new(1440.0, 900.0);
    readout.panel.solve();

    let choices = AddChoices::none();
    let row = master(
        &ctx,
        readout.panel.layout(),
        Some(1.0),
        Some(&view_chain),
        &choices,
    )
    .expect("master bay renders with chain slot");

    assert_eq!(row.slots.len(), 1);
    let slot_row = &row.slots[0];
    assert_eq!(slot_row.words, "zoom blur");
    assert_eq!(slot_row.params.len(), reading[0].params.len());

    // 5. Test parameter knob grab and dragging
    let param_row = &slot_row.params[0];
    let knob_point = Point::new(
        param_row.fader.knob.center().x,
        param_row.fader.knob.center().y,
    );
    let grab = row
        .grab(knob_point)
        .expect("knob hit-test grabs the parameter fader");

    assert_eq!(
        grab.knob(),
        karakuri_console::panel::Knob::Chain {
            at: 0,
            key: param_row.key.clone(),
            range: param_row.range,
        }
    );

    // 6. Test updating chain parameter via present.set_chain_params
    let mut updated_params = std::collections::BTreeMap::new();
    let new_val = param_row.range[1];
    updated_params.insert(param_row.key.clone(), new_val);
    let shape = vec![(proc_address.clone(), None)];
    let ok = present.set_chain_params(&gpu.queue, &shape, &[updated_params]);
    assert!(ok, "in-place parameter update succeeded");

    let updated_reading = present.chain_reading();
    let updated_p = updated_reading[0]
        .params
        .iter()
        .find(|p| p.key == param_row.key)
        .expect("param exists");
    assert_eq!(updated_p.value, new_val);

    // 7. Test remove button hit-test
    let remove_point = Point::new(slot_row.remove.center().x, slot_row.remove.center().y);
    let op = row.chip(remove_point);
    assert_eq!(op, Some(Operation::RemoveChainEffect { at: 0 }));

    // 8. Test clearing the chain
    let empty_chain = build_chain(&gpu.device, present.chain_layout(), &[], &|addr| {
        resolve_procedure(None, addr)
    })
    .expect("empty chain compiles");
    drop(present.set_chain(&gpu.device, &gpu.queue, empty_chain));
    assert!(present.chain_reading().is_empty());

    let empty_view = crate::bridge::chain_view(&present, &[]);
    let empty_row = master(
        &ctx,
        readout.panel.layout(),
        Some(1.0),
        Some(&empty_view),
        &choices,
    )
    .expect("master bay renders with empty chain");
    assert!(empty_row.slots.is_empty());
    assert!(empty_row.add.is_some());
}
