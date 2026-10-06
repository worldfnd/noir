//! The `shared` module contains simple types which are using in multiple of Noir's IRs.
//!
//! This is done to avoid each IR from needing to have its own definition of elementary types
//! while avoiding one IR being embedded within another.

mod foreign_calls;
mod integer_width;
mod signedness;
mod visibility;

pub use foreign_calls::ForeignCall;
pub use integer_width::{
    LOWERABLE_INTEGER_TYPES, MAX_INTEGER_WIDTH, is_legal_integer_width, is_lowerable_integer_width,
    parse_integer_type_name,
};
pub use signedness::Signedness;
pub use visibility::Visibility;
