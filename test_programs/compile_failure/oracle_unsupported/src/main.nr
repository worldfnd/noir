fn main() -> pub u32 {
    // Safety: the oracle is unconstrained
    unsafe {
        get_value()
    }
}

#[oracle(get_value)]
unconstrained fn get_value() -> u32 {}
