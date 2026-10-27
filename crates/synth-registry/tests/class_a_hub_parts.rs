// SPDX-License-Identifier: Apache-2.0

//! The Class A Protected Power Hub's parts name a KiCad symbol and carry the
//! ratings and decoupling the power checks read, not only a pin list.

use std::path::Path;

fn part(id: &str) -> synth_registry::Part {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../registry/parts");
    synth_registry::load_dir(&dir)
        .expect("seed registry must load")
        .iter()
        .find(|(k, _)| k.as_str() == id)
        .unwrap_or_else(|| panic!("{id} is missing"))
        .1
        .clone()
}

fn number(part: &synth_registry::Part, pin: &str) -> String {
    part.pins
        .iter()
        .find(|p| p.name == pin)
        .unwrap_or_else(|| panic!("{} has no pin {pin}", part.id))
        .number
        .0
        .clone()
}

#[test]
fn every_hub_part_with_a_kicad_symbol_names_it() {
    // `mp2307_buck_ep` is the exception: KiCad has no MP2307 symbol, and the
    // closest (MP2303ADN) has no exposed-pad pin, which this part must declare.
    for id in [
        "header_1x8",
        "bzt52c3v3",
        "l_power_10uh_6045",
        "tps22918_sot23_6",
    ] {
        let p = part(id);
        assert!(
            p.kicad_symbol.as_deref().is_some_and(|s| s.contains(':')),
            "{id} has no kicad_symbol"
        );
    }
}

#[test]
fn the_load_switch_follows_the_dbv_pinout_and_replaces_the_conflicting_entry() {
    let p = part("tps22918_sot23_6");
    for (pin, n) in [
        ("vin", "1"),
        ("gnd", "2"),
        ("on", "3"),
        ("ct", "4"),
        ("qod", "5"),
        ("vout", "6"),
    ] {
        assert_eq!(number(&p, pin), n, "{pin}");
    }
    let oc = p
        .operating_conditions
        .as_ref()
        .expect("operating conditions");
    assert_eq!(oc.max_voltage_v, Some(5.5));
    assert!(p.required_decoupling.iter().any(|d| d.net == "vin"));
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../registry/parts");
    let registry = synth_registry::load_dir(&dir).unwrap();
    assert!(
        !registry
            .iter()
            .any(|(k, _)| k.as_str() == "tps22918_loadswitch"),
        "the entry that put VOUT on pad 5 is gone"
    );
}

#[test]
fn the_buck_regulator_declares_its_range_and_input_capacitors() {
    let p = part("mp2307_buck_ep");
    let oc = p
        .operating_conditions
        .as_ref()
        .expect("operating conditions");
    assert_eq!(oc.min_voltage_v, Some(4.75));
    assert_eq!(oc.max_voltage_v, Some(23.0));
    assert_eq!(oc.max_current_ma, Some(3000.0));
    let input: u32 = p
        .required_decoupling
        .iter()
        .filter(|d| d.net == "in")
        .map(|d| d.count)
        .sum();
    assert!(input >= 3, "10 uF x2 and 100 nF on IN, got {input}");
    assert_eq!(number(&p, "gnd_ep"), "9");
    assert!(
        p.kicad_footprint
            .as_deref()
            .unwrap()
            .contains("EP2.41x3.3mm"),
        "the exposed pad matches the datasheet's 3.15-3.45 x 2.26-2.56 mm"
    );
}

#[test]
fn the_inductor_carries_the_datasheet_current_rating() {
    let p = part("l_power_10uh_6045");
    let oc = p
        .operating_conditions
        .as_ref()
        .expect("operating conditions");
    assert_eq!(oc.max_current_ma, Some(3200.0));
    assert!(p.description.as_deref().unwrap().contains("Isat 4.6 A"));
}
