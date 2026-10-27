// SPDX-License-Identifier: Apache-2.0

//! The four active parts of the Class A I2C sensor node carry what the checks
//! read: bus capabilities, required supply and ground pins, an operating range
//! and decoupling. A part that merely loads would let a board omit any of them.

use std::path::Path;
use synth_registry::{load_dir, PinCapability};

fn registry() -> synth_registry::Registry {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../registry/parts");
    load_dir(&dir).expect("seed registry must load")
}

fn part(id: &str) -> synth_registry::Part {
    registry()
        .iter()
        .find(|(k, _)| k.as_str() == id)
        .unwrap_or_else(|| panic!("{id} is missing"))
        .1
        .clone()
}

fn caps(part: &synth_registry::Part, pin: &str) -> Vec<PinCapability> {
    part.pins
        .iter()
        .find(|p| p.name == pin)
        .unwrap_or_else(|| panic!("{} has no pin {pin}", part.id))
        .capabilities
        .clone()
}

fn required(part: &synth_registry::Part, pin: &str) -> bool {
    part.pins.iter().find(|p| p.name == pin).unwrap().required
}

fn assert_electrical_metadata(part: &synth_registry::Part, supplies: &[&str]) {
    let oc = part
        .operating_conditions
        .as_ref()
        .unwrap_or_else(|| panic!("{} has no operating conditions", part.id));
    assert!(oc.min_voltage_v.is_some() && oc.max_voltage_v.is_some());
    assert!(oc.max_current_ma.is_some());
    for pin in supplies {
        assert!(
            required(part, pin),
            "{} pin {pin} must be required",
            part.id
        );
        assert!(
            part.required_decoupling.iter().any(|d| d.net == *pin)
                || *pin == "vss"
                || *pin == "gnd",
            "{} must declare decoupling on {pin}",
            part.id
        );
    }
}

#[test]
fn the_eeprom_is_an_i2c_device_with_required_supply_and_address_pins() {
    let p = part("24lc256_soic8");
    assert!(caps(&p, "sda").contains(&PinCapability::I2cSda));
    assert!(caps(&p, "scl").contains(&PinCapability::I2cScl));
    for pin in ["a0", "a1", "a2", "wp", "vss", "vcc"] {
        assert!(required(&p, pin), "{pin}");
    }
    assert_electrical_metadata(&p, &["vcc", "vss"]);
}

#[test]
fn the_environment_sensor_declares_both_buses_it_claims() {
    let p = part("bme680_lga8");
    assert!(caps(&p, "sda").contains(&PinCapability::I2cSda));
    assert!(caps(&p, "scl").contains(&PinCapability::I2cScl));
    assert!(caps(&p, "sda").contains(&PinCapability::SpiMosi));
    assert!(caps(&p, "scl").contains(&PinCapability::SpiSck));
    assert!(caps(&p, "csb").contains(&PinCapability::SpiCs));
    assert!(caps(&p, "sdo").contains(&PinCapability::SpiMiso));
    for pin in ["csb", "sdo", "vddio", "gnd2"] {
        assert!(required(&p, pin), "{pin}");
    }
    assert_electrical_metadata(&p, &["vdd", "vddio", "gnd"]);
}

#[test]
fn the_usb_bridge_declares_its_uart_and_required_pins() {
    let p = part("ch340c");
    assert!(caps(&p, "txd").contains(&PinCapability::UartTx));
    assert!(caps(&p, "rxd").contains(&PinCapability::UartRx));
    for pin in ["vcc", "gnd", "v3", "ud_plus", "ud_minus"] {
        assert!(required(&p, pin), "{pin}");
    }
    assert_electrical_metadata(&p, &["vcc", "gnd"]);
}

#[test]
fn the_mcu_declares_gpio_buses_reset_and_boot() {
    let p = part("stm32g030f6p6");
    assert!(caps(&p, "nrst").contains(&PinCapability::Reset));
    assert!(caps(&p, "pa14_pa15").contains(&PinCapability::BootMode));
    assert!(caps(&p, "pa2").contains(&PinCapability::UartTx));
    assert!(caps(&p, "pa3").contains(&PinCapability::UartRx));
    assert!(caps(&p, "pa9_pa11").contains(&PinCapability::I2cScl));
    assert!(caps(&p, "pa10_pa12").contains(&PinCapability::I2cSda));
    assert!(caps(&p, "pa5").contains(&PinCapability::SpiSck));
    let gpio = p
        .pins
        .iter()
        .filter(|pin| pin.capabilities.contains(&PinCapability::Gpio))
        .count();
    assert!(gpio >= 17, "expected every port pin to be GPIO, got {gpio}");
    for pin in ["vdd", "vss", "nrst"] {
        assert!(required(&p, pin), "{pin}");
    }
    assert_electrical_metadata(&p, &["vdd", "vss"]);
}
