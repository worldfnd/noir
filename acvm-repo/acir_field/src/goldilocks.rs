use ark_ff::fields::{Fp64, MontBackend, MontConfig};

/// The Goldilocks prime field, p = 2^64 - 2^32 + 1.
///
/// Used by STARK-friendly proving backends (e.g. Plonky2/Plonky3). Unlike bn254,
/// `p` does not exceed `u64::MAX`, so a 64-bit integer type can hold values that
/// no longer fit in this field — this is the assumption the frontend must not bake in.
#[derive(MontConfig)]
#[modulus = "18446744069414584321"]
#[generator = "7"]
pub struct GoldilocksConfig;

pub type Goldilocks = Fp64<MontBackend<GoldilocksConfig, 1>>;

#[cfg(test)]
mod tests {
    use crate::{AcirField, FieldElement};

    #[test]
    fn modulus_and_bits() {
        assert_eq!(FieldElement::max_num_bits(), 64);
        assert_eq!(FieldElement::modulus().to_string(), "18446744069414584321");
    }

    #[test]
    fn wraps_at_modulus() {
        // p - 1 + 1 == 0 mod p
        let p_minus_1 = FieldElement::zero() - FieldElement::one();
        assert!((p_minus_1 + FieldElement::one()).is_zero());
        // 2^64 reduces to 2^32 - 1 (since 2^64 = 2^32 - 1 mod p)
        let two_64 = FieldElement::from(u64::MAX) + FieldElement::one();
        assert_eq!(two_64.to_u128(), (1u128 << 32) - 1);
    }
}
