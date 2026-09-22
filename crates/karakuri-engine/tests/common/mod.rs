//! Shared fixtures and helper utilities for `karakuri-engine` integration tests.

#![allow(dead_code)]

use karakuri_ir::typed::Checked;

/// Parse, check, and estimate an IR procedural shader string, panicking with
/// formatted diagnostics on any error.
pub fn compile(src: &str) -> Checked {
    let proc = karakuri_ir::parse(src).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    let checked =
        karakuri_ir::check::check(&proc).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    karakuri_ir::cost::estimate(&checked).unwrap_or_else(|e| panic!("{}", render(&e, src)));
    checked
}

/// Render a collection of IR errors with source context.
pub fn render(errs: &[karakuri_ir::IrError], src: &str) -> String {
    errs.iter()
        .map(|e| e.render(src))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Decode a 16-bit half-precision floating point number (IEEE 754 binary16) to `f32`.
pub fn f16(bits: u16) -> f32 {
    let sign = f32::from_bits(u32::from(bits & 0x8000) << 16);
    let exp = (bits >> 10) & 0x1f;
    let mant = u32::from(bits & 0x3ff);
    let v = match exp {
        0 => f32::from_bits(mant << 13) * 2.0f32.powi(-112),
        0x1f => f32::from_bits(0x7f80_0000 | (mant << 13)),
        _ => f32::from_bits(((u32::from(exp) + 112) << 23) | (mant << 13)),
    };
    f32::from_bits(v.to_bits() | sign.to_bits())
}

/// Encode an `f32` value into an exact `u16` half-precision float representation,
/// handling special values (`NaN`, `+Inf`, `-Inf`).
pub fn f32_to_f16(x: f32) -> u16 {
    if x == 0.0 {
        return 0;
    }
    if x.is_nan() {
        return 0x7e00;
    }
    if x.is_infinite() {
        return if x > 0.0 { 0x7c00 } else { 0xfc00 };
    }
    let bits = x.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exponent = ((bits >> 23) & 0xff) as i32 - 127;
    let mantissa = bits & 0x007f_ffff;
    assert!(
        (-14..=15).contains(&exponent) && mantissa & 0x1fff == 0,
        "{x} is not exactly representable as an f16, so this test would be measuring \
         a rounding rather than a reduction"
    );
    sign | (((exponent + 15) as u16) << 10) | (mantissa >> 13) as u16
}
