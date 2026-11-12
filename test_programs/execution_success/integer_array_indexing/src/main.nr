global ARRAY_LEN: u32 = 3;

fn main(arr: [u8; ARRAY_LEN], x: u32) -> pub u8 {
    let mut value = arr[ARRAY_LEN - 1];

    value += arr[0u32];
    value += arr[1u32];

    value + x as u8
}
