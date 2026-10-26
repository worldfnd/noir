fn main(args: [u8; 2]) {
    // Safety: n/a
    unsafe { store(args) };
    // Dummy test to remove the 'underconstraint bug'
    assert(args[0] as Field + args[1] as Field != 0);
}

pub unconstrained fn store(_: [u8]) {}
