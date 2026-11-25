// `inner` is written as a constrained function, but it is only reached from the unconstrained
// `hint`, so it is compiled as unconstrained and its proof verification cannot be lowered.
fn inner(vk: [Field; 1], proof: [Field; 1], pi: [Field; 1], kh: Field) {
    std::verify_proof_with_type(vk, proof, pi, kh, 0);
}

unconstrained fn hint(vk: [Field; 1], proof: [Field; 1], pi: [Field; 1], kh: Field) {
    inner(vk, proof, pi, kh);
}

fn main(vk: [u8; 1], proof: [u8; 1], pi: [u8; 1], kh: u8) {
    let vk = vk.map(|v| Field::from(v));
    let proof = proof.map(|v| Field::from(v));
    let pi = pi.map(|v| Field::from(v));
    let kh = Field::from(kh);

    // Safety: `hint` returns nothing; the program only exercises the compile-time rejection.
    unsafe { hint(vk, proof, pi, kh) };
}
