use std::embedded_curve_ops::{EmbeddedCurvePoint, EmbeddedCurveScalar, multi_scalar_mul};

// A multi-scalar multiplication against the point at infinity must still range-validate
// the witness scalar limbs. `hi` here is 2^128, which is out of range for a scalar limb,
// so execution must fail rather than silently eliding the validation.
fn main(lo: u8, hi: [u8; 32]) {
    let point = EmbeddedCurvePoint::point_at_infinity();
    let scalar = EmbeddedCurveScalar::new(lo as Field, Field::from_le_bytes_checked(hi));
    let result = multi_scalar_mul([point], [scalar]);
    assert(result.is_infinite());
}
