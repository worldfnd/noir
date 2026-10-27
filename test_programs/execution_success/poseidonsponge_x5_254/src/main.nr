use poseidon::poseidon;

fn main(x: [u8; 7]) {
    // Test optimized sponge
    let result = poseidon::bn254::sponge(x.map(|x| x as Field));

    assert(result == 0x080ae1669d62f0197190573d4a325bfb8d8fc201ce3127cbac0c47a7ac81ac48);
}
