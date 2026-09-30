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

    // 9. Verify drop landing covers the master bay chain body even when empty
    let list = empty_row.list.expect("empty master bay has a drop list");
    let mid_empty_body = Point::new(list.center().x, list.center().y);
    assert_eq!(empty_row.dropped(mid_empty_body), Some(list));
}

#[test]
fn resolve_procedure_and_shipped_source_resolves_by_name_and_hash() {
    let zoom_blur_src = shipped::ZOOM_BLUR;
    let proc_address = shipped::address(zoom_blur_src);

    // Resolve by address
    assert_eq!(
        resolve_procedure(None, &proc_address).as_deref(),
        Some(zoom_blur_src)
    );
    assert_eq!(shipped::source(&proc_address), Some(zoom_blur_src));
    assert_eq!(shipped::name_of(&proc_address), Some("zoom blur"));

    // Resolve by name
    assert_eq!(
        resolve_procedure(None, "zoom_blur").as_deref(),
        Some(zoom_blur_src)
    );
    assert_eq!(shipped::source("zoom_blur"), Some(zoom_blur_src));
    assert_eq!(shipped::name_of("zoom_blur"), Some("zoom blur"));
}

#[test]
fn starring_preset_procedure_adds_to_favourites_and_shows_in_my_sets() {
    let test_dir = std::env::temp_dir().join(format!("karakuri_star_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&test_dir);
    let store_root = test_dir.join("store");
    let presets_root = test_dir.join("presets");
    std::fs::create_dir_all(&presets_root).expect("create presets");

    // Put a preset procedure into presets_root
    let sample_l5 = r#"kind L5
params
  amount 0.5 [0, 1]
frag
  return texture(src, uv);
"#;
    std::fs::write(presets_root.join("sample_fx.kir"), sample_l5).expect("write sample_fx");

    let presets = karakuri_environment::places::Presets {
        dir: presets_root.clone(),
        found: karakuri_environment::places::Found::Given,
    };

    // Initialize Store
    let store = karakuri_store::Store::open(&store_root).expect("open store");
    drop(store);

    // Star the preset procedure via bridge favourite()
    let star_op = Operation::SetFavourite {
        id: "sample_fx".to_owned(),
        favourite: true,
    };
    let response = favourite(&store_root, Some(&presets), Asked::Operator, &star_op)
        .expect("favourite answered");
    assert!(response.contains("is starred"), "{response}");

    // Also star a built-in shipped procedure ("zoom_blur")
    let zoom_op = Operation::SetFavourite {
        id: "zoom_blur".to_owned(),
        favourite: true,
    };
    let response_zoom = favourite(&store_root, Some(&presets), Asked::Operator, &zoom_op)
        .expect("zoom_blur favourite answered");
    assert!(response_zoom.contains("is starred"), "{response_zoom}");

    // Read back in listing under Scope::MySets
    let mut view = View::new(karakuri_console::room::Room::Day);
    view.scopes = karakuri_console::view::Scope::ALL.to_vec();
    assert!(view.select_scope(karakuri_console::view::Scope::MySets));

    listing(&mut view, &store_root, Some(&presets), None, None);

    assert!(
        view.starred.contains("sample_fx"),
        "starred set should contain sample_fx: {:?}",
        view.starred
    );
    assert!(
        view.starred.contains("zoom_blur"),
        "starred set should contain zoom_blur: {:?}",
        view.starred
    );
    assert!(
        view.library.contains(&"sample_fx".to_owned()),
        "MySets should list starred preset procedure sample_fx: {:?}",
        view.library
    );
    assert!(
        view.library.contains(&"zoom_blur".to_owned()),
        "MySets should list starred shipped procedure zoom_blur: {:?}",
        view.library
    );

    // Verify unstarring removes them from MySets
    let unstar_op = Operation::SetFavourite {
        id: "sample_fx".to_owned(),
        favourite: false,
    };
    favourite(&store_root, Some(&presets), Asked::Operator, &unstar_op).expect("unstar answered");

    listing(&mut view, &store_root, Some(&presets), None, None);

    assert!(
        !view.library.contains(&"sample_fx".to_owned()),
        "MySets should not list unstarred procedure sample_fx"
    );

    let _ = std::fs::remove_dir_all(&test_dir);
}

#[test]
fn chain_swap_worker_compilation_and_installation() {
    let gpu = pollster::block_on(Gpu::new(None)).expect("headless gpu");
    let mut chain_swap = karakuri_engine::ChainSwap::new(&gpu.device, &gpu.queue);
    let mut present =
        karakuri_engine::Present::new(&gpu.device, karakuri_engine::Present::HDR_FORMAT, 1280, 720);
    let zoom_blur_src = shipped::ZOOM_BLUR;
    let proc_address = shipped::address(zoom_blur_src);
    let slot_spec = karakuri_engine::SlotSpec {
        procedure: proc_address.clone(),
        cut: None,
        params: std::collections::BTreeMap::new(),
    };
    let resolve = |addr: &str| resolve_procedure(None, addr);
    let res = karakuri_environment::mix::apply_chain(
        &mut chain_swap,
        &mut present,
        &gpu.queue,
        &[slot_spec],
        &resolve,
    );
    assert!(res.is_ok(), "{:?}", res);

    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        chain_swap.begin_frame(&mut present, &gpu.device, &gpu.queue);
        for _event in chain_swap.events() {}
        if !present.chain_reading().is_empty() {
            break;
        }
    }
    assert!(
        !present.chain_reading().is_empty(),
        "Chain was not installed into present after worker finished!"
    );

    let view_chain = crate::bridge::chain_view(&present, &[]);
    assert_eq!(view_chain.slots.len(), 1);

    let ctx = crate::tests::drawn_once();
    let choices = AddChoices::none();

    for (w, h) in [
        (1440.0, 900.0),
        (1244.0, 658.5),
        (1920.0, 1080.0),
        (1024.0, 768.0),
    ] {
        let mut readout = crate::readout::Readout::new(w, h);
        readout.panel.solve();
        let row = master(
            &ctx,
            readout.panel.layout(),
            Some(1.0),
            Some(&view_chain),
            &choices,
        )
        .expect("master bay renders with chain slot");
        assert_eq!(
            row.slots.len(),
            1,
            "Slot should be rendered even in tight viewports ({w}x{h})"
        );
        if h >= 768.0 {
            assert!(
                !row.slots[0].params.is_empty(),
                "Parameters should fit within viewport ({w}x{h})"
            );
        }
    }
}
