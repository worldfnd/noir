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
    (point.x as u8, point.y as u8)
}
