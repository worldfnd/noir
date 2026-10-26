global ARRAY_LEN: u32 = 3;

fn main(arr: [u8; ARRAY_LEN], x: u32) -> pub u8 {
    let mut value = arr[ARRAY_LEN - 1] as Field;

    value += arr[0u32] as Field;
    value += arr[1u32] as Field;

    (value + x as Field) as u8
}
