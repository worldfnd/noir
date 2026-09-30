fn main() {
    let array = [1, 2, 3];
    let _ = std::mem::array_refcount(array);
    let _ = std::mem::vector_refcount(array.as_vector());
}
