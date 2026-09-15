#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, PartialOrd, Ord)]
pub enum Signedness {
    Unsigned,
    Signed,
}

impl Signedness {
    pub fn is_signed(&self) -> bool {
        match self {
            Signedness::Unsigned => false,
            Signedness::Signed => true,
        }
    }

    /// The letter that starts the name of an integer type of this signedness: `u` or `i`.
    pub fn type_name_prefix(&self) -> &'static str {
        match self {
            Signedness::Unsigned => "u",
            Signedness::Signed => "i",
        }
    }
}
