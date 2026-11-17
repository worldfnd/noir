use std::convert::UnsafeFrom;
// https://github.com/noir-lang/noir/issues/7744

use std::embedded_curve_ops::{embedded_curve_add, EmbeddedCurvePoint};

// is_active = false
fn main(is_active: bool) -> pub (u8, u8) {
    let bad = EmbeddedCurvePoint::new(0, 5);
    let point = if is_active {
        embedded_curve_add(bad, bad)
    } else {
        bad
    };
    (u8::unsafe_from(point.x), u8::unsafe_from(point.y))
}
