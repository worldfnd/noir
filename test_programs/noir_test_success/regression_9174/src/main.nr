global G_A: [u32] = [].as_vector();
fn main(a: bool, b: bool, c: u32) -> pub (u32, u32) {
    if a {
        (
            {
                let i = G_A;
                i[(2050918985_u32 % i.len())]
            },
            // Safety: test
            unsafe {
                func_1_proxy(
                    if a {
                        if b {
                            func_1_proxy(0)
                        } else {
                            766482358_u32
                        }
                    } else {
                        1568313658_u32
                    },
                )
            },
        )
    } else {
        (
            {
                let k = G_A;
                k[(779011912_u32 % k.len())]
            }, c,
        )
    }
}

unconstrained fn func_1_proxy(_a: u32) -> u32 {
    0
}

#[test(should_fail_with = "attempt to calculate the remainder with a divisor of zero")]
fn fuzz_main(a: bool, b: bool) -> (u32, u32) {
    main(a, b, 0x49e9b512)
}
