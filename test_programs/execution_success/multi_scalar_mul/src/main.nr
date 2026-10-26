// This test provides a basic implementation of a MSM in Noir, that allows us to check
// performance improvements and regressions.
use std::embedded_curve_ops::embedded_curve_add;
use std::embedded_curve_ops::EmbeddedCurvePoint;

// `main` must be marked unconstrained as the function uses `break` internally
unconstrained fn main(
    points: [(u8, [u8; 32]); 5],
    scalars: [[u8; 32]; 5],
) -> pub ([u8; 32], [u8; 32]) {
    double_then_add_msm(points, scalars)
}

unconstrained fn double_then_add_msm<let N: u32>(
    p: [(u8, [u8; 32]); N],
    scalars: [[u8; 32]; N],
) -> ([u8; 32], [u8; 32]) {
    // EmbeddedCurveScalar are two 128-bit numbers
    let mut acc = EmbeddedCurvePoint::point_at_infinity();
    for i in 0..N {
        // These should probably be EmbeddedCurveScalars
        // let full_scalar: Field = scalars[i].hi * 2.pow_32(128) + scalars[i].lo;
        let full_scalar = Field::from_le_bytes_checked(scalars[i]);
        // If the scalar is zero we won't add anything to acc
        if full_scalar == 0 {
            continue;
        }
        let full_scalar_bits: [bool; 254] = full_scalar.to_be_bits();
        let mut index_of_msb = 0;
        // Iterates in BE
        for j in 0..254 {
            if full_scalar_bits[j] {
                index_of_msb = j;
                break;
            }
        }

        let temp = EmbeddedCurvePoint::new(p[i].0 as Field, Field::from_le_bytes_checked(p[i].1));
        let mut res = temp;
        // traversing from second MSB to LSB
        for j in (index_of_msb + 1)..(254) {
            // Double
            res = embedded_curve_add(res, res);
            // Add
            if full_scalar_bits[j] {
                res = embedded_curve_add(res, temp);
            }
        }

        acc = embedded_curve_add(acc, res);
    }
    (acc.x.to_le_bytes(), acc.y.to_le_bytes())
}
