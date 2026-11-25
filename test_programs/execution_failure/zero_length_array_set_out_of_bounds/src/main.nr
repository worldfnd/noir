fn main(idx: u32) -> pub ([[u8; 0]; 4], u32) {
    let mut slots: [[u8; 0]; 4] = [[], [], [], []];
    slots[idx] = [];
    (slots, idx)
}
