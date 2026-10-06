//! Compiler frontend options and unstable feature flags.

use std::str::FromStr;

use acvm::{FieldConfig, FieldId};

use crate::node_interner::FieldGates;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum UnstableFeature {
    Enums,
    TraitAsType,
}

impl std::fmt::Display for UnstableFeature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Enums => write!(f, "enums"),
            Self::TraitAsType => write!(f, "trait_as_type"),
        }
    }
}

impl FromStr for UnstableFeature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "enums" => Ok(Self::Enums),
            "trait_as_type" => Ok(Self::TraitAsType),
            other => Err(format!("Unknown unstable feature '{other}'")),
        }
    }
}

/// Generic options struct meant to resolve to `ElaboratorOptions` below when
/// we can resolve a file path to a file id later. This generic struct is used
/// so that `FrontendOptions` doesn't need to duplicate fields and methods with `ElaboratorOptions`.
#[derive(Copy, Clone, Debug)]
pub struct GenericOptions<'a, T> {
    /// The scope of --debug-comptime, or None if unset
    pub debug_comptime_in_file: Option<T>,

    /// Unstable compiler features that were explicitly enabled. Any unstable features
    /// that are not in this list result in an error when used.
    pub enabled_unstable_features: &'a [UnstableFeature],

    /// Deny crates from requiring unstable features.
    pub disable_required_unstable_features: bool,

    /// The field the compilation runs under.
    pub field: FieldConfig,

    /// Benchmark mode for bn254 builds: where a module keeps an item gated `bn254` and a twin
    /// of the same kind and name gated `not(bn254)`, compile the twin. Off by default.
    pub generic_builtins: bool,
}

/// Options from `nargo_cli` that need to be passed down to the elaborator
pub(crate) type ElaboratorOptions<'a> = GenericOptions<'a, fm::FileId>;

/// This is the unresolved version of `ElaboratorOptions`
/// CLI options that need to be passed to the compiler frontend (the elaborator).
pub type FrontendOptions<'a> = GenericOptions<'a, &'a str>;

impl<T> GenericOptions<'_, T> {
    /// A sane default of frontend options for running tests
    pub fn test_default() -> GenericOptions<'static, T> {
        GenericOptions {
            debug_comptime_in_file: None,
            enabled_unstable_features: &[UnstableFeature::Enums],
            disable_required_unstable_features: true,
            field: FieldConfig::linked(),
            generic_builtins: false,
        }
    }

    /// How `#[field(..)]` gates are decided under these options. `generic_builtins` acts in a
    /// bn254 build only: there `not(bn254)` marks a field-generic twin or its helper, while under
    /// another field `not(<field>)` marks an item the field's range excludes, which no benchmark
    /// may bring back.
    pub fn field_gates(&self) -> FieldGates {
        let generic_builtins = self.generic_builtins && self.field.id() == FieldId::Bn254;
        FieldGates { field: self.field, generic_builtins }
    }
}
