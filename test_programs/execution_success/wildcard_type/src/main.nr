struct bar {
    enable: [bool; 4],
    data: [u8; 2],
    pad: u32,
}

fn main(enable: [u8; 2]) -> pub [u8; 4] {
    let mut result = [0; 4];
    let a: [_; 4] = foo(enable[1]);
    for i in 0..4 {
        result[i] = a[i].data[i % 2];
    }
    result
}

fn foo(x: u8) -> [bar; 4] {
    [
        bar { enable: [true, true, false, false], data: [x, x + 1], pad: 0 },
        bar { enable: [true, false, false, false], data: [x + 2, x + 7], pad: 0 },
        bar { enable: [true, true, false, true], data: [x + 3, x + 5], pad: 0 },
        bar { enable: [false, false, false, false], data: [x + 4, x - 1], pad: 0 },
    ]
}
