//! Field metadata independent of the linked arithmetic backend. Exhaustive matches require each field to define its properties.

use std::fmt::Display;
use std::str::FromStr;
use std::sync::LazyLock;

use num_bigint::BigUint;

/// A field identity with a stable wire code. Use `from_code` to reject unknown codes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum FieldId {
    Bn254 = 0,
    Goldilocks = 1,
    Bls12_381 = 2,
}

impl FieldId {
    pub const ALL: [FieldId; 3] = [FieldId::Bn254, FieldId::Goldilocks, FieldId::Bls12_381];

    pub fn code(self) -> u32 {
        self as u32
    }

    pub fn from_code(code: u32) -> Option<FieldId> {
        Self::ALL.into_iter().find(|id| id.code() == code)
    }

    pub fn name(self) -> &'static str {
        match self {
            FieldId::Bn254 => "bn254",
            FieldId::Goldilocks => "goldilocks",
            FieldId::Bls12_381 => "bls12_381",
        }
    }

    pub fn from_name(name: &str) -> Option<FieldId> {
        Self::ALL.into_iter().find(|id| id.name() == name)
    }

    /// The field whose arithmetic this crate is built with, selected by its cargo features.
    pub fn linked() -> FieldId {
        cfg_if::cfg_if! {
            if #[cfg(feature = "goldilocks")] {
                FieldId::Goldilocks
            } else if #[cfg(feature = "bls12_381")] {
                FieldId::Bls12_381
            } else {
                FieldId::Bn254
            }
        }
    }
}

impl Display for FieldId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for FieldId {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::from_name(name).ok_or_else(|| {
            let names: Vec<_> = Self::ALL.iter().map(|id| id.name()).collect();
            format!("unknown field '{name}'; expected one of {}", names.join(", "))
        })
    }
}

/// A curve whose base field is a supported field, so its group operations are available as builtins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmbeddedCurve {
    Grumpkin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldConfig {
    id: FieldId,
}

static BN254_MODULUS: LazyLock<BigUint> = LazyLock::new(|| {
    parse_decimal("21888242871839275222246405745257275088548364400416034343698204186575808495617")
});
static GOLDILOCKS_MODULUS: LazyLock<BigUint> =
    LazyLock::new(|| parse_decimal("18446744069414584321"));
static BLS12_381_MODULUS: LazyLock<BigUint> = LazyLock::new(|| {
    parse_decimal("52435875175126190479447740508185965837690552500527637822603658699938581184513")
});

fn parse_decimal(digits: &str) -> BigUint {
    BigUint::parse_bytes(digits.as_bytes(), 10).expect("a field modulus is written in decimal")
}

impl FieldConfig {
    pub const fn new(id: FieldId) -> FieldConfig {
        FieldConfig { id }
    }

    pub fn linked() -> FieldConfig {
        FieldConfig::new(FieldId::linked())
    }

    pub fn id(&self) -> FieldId {
        self.id
    }

    pub fn name(&self) -> &'static str {
        self.id.name()
    }

    /// Whether the argument of a `#[field(..)]` attribute names this field: either by name, or by its modulus written in decimal or `0x` hexadecimal.
    pub fn matches_field_attribute(&self, argument: &str) -> bool {
        let modulus = match argument.strip_prefix("0x") {
            Some(hex) => BigUint::parse_bytes(hex.as_bytes(), 16),
            None => BigUint::parse_bytes(argument.as_bytes(), 10),
        };
        match modulus {
            Some(modulus) => modulus == *self.modulus(),
            None => argument == self.name(),
        }
    }

    pub fn modulus(&self) -> &'static BigUint {
        match self.id {
            FieldId::Bn254 => &BN254_MODULUS,
            FieldId::Goldilocks => &GOLDILOCKS_MODULUS,
            FieldId::Bls12_381 => &BLS12_381_MODULUS,
        }
    }

    /// The bit length of the modulus.
    pub fn num_bits(&self) -> u32 {
        u32::try_from(self.modulus().bits()).expect("a field modulus fits in u32 bits")
    }

    /// Serialized element width in bytes, rounded to the backend's 64-bit limbs.
    pub fn num_bytes(&self) -> u32 {
        self.num_bits().div_ceil(64) * 8
    }

    /// The curve available to the embedded-curve builtins.
    pub fn embedded_curve(&self) -> Option<EmbeddedCurve> {
        match self.id {
            FieldId::Bn254 => Some(EmbeddedCurve::Grumpkin),
            FieldId::Goldilocks | FieldId::Bls12_381 => None,
        }
    }

    /// Whether a native Poseidon2 builtin exists; Noir implementations are independent of this.
    pub fn has_poseidon2_permutation(&self) -> bool {
        match self.id {
            FieldId::Bn254 => true,
            FieldId::Goldilocks | FieldId::Bls12_381 => false,
        }
    }

    /// Whether every value of a `width`-bit unsigned type is below the modulus, so the type can be carried in one field element without loss. A `k`-bit modulus lies strictly between `2^(k-1)` and `2^k`, so this holds exactly for widths up to `k - 1`.
    pub fn fits_unsigned(&self, width: u32) -> bool {
        width < self.num_bits()
    }
}

#[cfg(test)]
mod tests {
    use crate::{AcirField, EmbeddedCurve, FieldConfig, FieldElement, FieldId};

    const GOLDILOCKS_MODULUS: &str = "18446744069414584321";

    #[test]
    fn codes_round_trip_through_the_checked_decoder() {
        for id in FieldId::ALL {
            assert_eq!(FieldId::from_code(id.code()), Some(id));
        }
        assert_eq!(FieldId::Bn254.code(), 0);
        assert_eq!(FieldId::from_code(FieldId::ALL.len() as u32), None);
    }

    #[test]
    fn names_round_trip_and_unknown_names_are_refused() {
        for id in FieldId::ALL {
            assert_eq!(FieldId::from_name(id.name()), Some(id));
            assert_eq!(id.name().parse::<FieldId>(), Ok(id));
            assert_eq!(id.to_string(), id.name());
        }
        assert_eq!(FieldId::from_name("bn-254"), None);
        let error = "bn-254".parse::<FieldId>().unwrap_err();
        assert!(error.contains("bn254") && error.contains("goldilocks"), "{error}");
    }

    #[test]
    fn every_row_states_its_modulus_and_width() {
        let widths = [
            (FieldId::Bn254, 254, 32),
            (FieldId::Goldilocks, 64, 8),
            (FieldId::Bls12_381, 255, 32),
        ];
        for (id, bits, bytes) in widths {
            let config = FieldConfig::new(id);
            assert_eq!(config.id(), id);
            assert_eq!(config.name(), id.name());
            assert_eq!(config.num_bits(), bits, "{id}");
            assert_eq!(config.num_bytes(), bytes, "{id}");
            assert!(config.fits_unsigned(bits - 1), "{id}: u{} fits", bits - 1);
            assert!(!config.fits_unsigned(bits), "{id}: u{bits} can exceed the modulus");
        }
        assert_eq!(FieldConfig::new(FieldId::Goldilocks).modulus().to_string(), GOLDILOCKS_MODULUS);
    }

    #[test]
    fn a_field_attribute_argument_matches_by_name_or_by_modulus() {
        let goldilocks = FieldConfig::new(FieldId::Goldilocks);
        assert!(goldilocks.matches_field_attribute("goldilocks"));
        assert!(goldilocks.matches_field_attribute(GOLDILOCKS_MODULUS));
        assert!(goldilocks.matches_field_attribute("0xffffffff00000001"));
        assert!(!goldilocks.matches_field_attribute("bn254"));
        assert!(!goldilocks.matches_field_attribute("23"));
        assert!(!goldilocks.matches_field_attribute("0x17"));

        let bn254 = FieldConfig::new(FieldId::Bn254);
        assert!(bn254.matches_field_attribute("bn254"));
        assert!(bn254.matches_field_attribute(&bn254.modulus().to_string()));
        assert!(!bn254.matches_field_attribute(GOLDILOCKS_MODULUS));
    }

    #[test]
    fn builtin_capabilities_are_field_specific() {
        let bn254 = FieldConfig::new(FieldId::Bn254);
        assert_eq!(bn254.embedded_curve(), Some(EmbeddedCurve::Grumpkin));
        assert!(bn254.has_poseidon2_permutation());

        for id in FieldId::ALL.into_iter().filter(|id| *id != FieldId::Bn254) {
            let config = FieldConfig::new(id);
            assert_eq!(config.embedded_curve(), None, "{id}");
            assert!(!config.has_poseidon2_permutation(), "{id}");
        }
    }

    /// Check the configured modulus against the arithmetic backend selected by this build.
    #[test]
    fn the_linked_row_agrees_with_the_linked_field_element() {
        let config = FieldConfig::linked();
        assert_eq!(config.id(), FieldId::linked());
        assert_eq!(*config.modulus(), FieldElement::modulus());
        assert_eq!(config.num_bits(), FieldElement::max_num_bits());
    }
}
