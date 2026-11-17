---
title: Type Casting and Coercions
description:
  Noir's explicit type casts and implicit type coercions
keywords:
  [
    noir,
    types,
    coercions,
    casts,
    as,
  ]
---

## Type Casting

You can explicitly convert between integer types using the `as` keyword:

```rust
let x: u32 = 100;
let y: u8 = x as u8;       // Cast u32 to u8 (truncates if value exceeds u8 range)
let z: u64 = x as u64;     // Cast u32 to u64
```

### Valid casts

The `as` keyword can be used to cast between the following types:

| From | To | Notes |
|------|----|-------|
| Any integer | Any other integer | Truncates to fit the target bit size |
| `bool` | Any integer, or `Field` | `false` is 0 and `true` is 1 |
| An integer literal | Any integer | The literal must fit the target type, as in `5 as u8` |

### Restrictions

- **`as` does not convert between `Field` and an integer type** -- this will produce a compiler error. Use one of the [conversions between `Field` and integers](#converting-between-field-and-integers) instead, which say what happens to a value that does not fit.
- **Non-numeric types** (arrays, strings, structs, tuples, etc.) cannot be cast to or from numeric types.
- **Casting to `bool`** is not supported. Compare with zero instead: `value != 0`

### Example

```rust
fn main() {
    let big: u32 = 300;
    let small = big as u8;    // Truncates: 300 does not fit in u8

    let signed: i32 = -1;
    let reinterpreted = signed as u32;  // The same bits read as unsigned: 4294967295

    // let field_val: Field = 42;
    // let bad = field_val as u64;  // Error: cannot cast `Field` as `u64`
}
```

### Converting between `Field` and integers

How many values a `Field` holds depends on the field of the proving backend, so a conversion between a `Field` and an integer says what it does with a value that does not fit:

| Conversion | Exists for | A value that does not fit |
|------------|-----------|---------------------------|
| `Field::from(x)` | integer types whose every value is a field element | Cannot happen |
| `Field::try_from(x)`, `u8::try_from(f)` | every integer type | Returns `None` |
| `Field::unsafe_from(x)`, `u8::unsafe_from(f)` | every integer type | Wraps: into a `Field` modulo the field, out of a `Field` to the low bits of the integer |

`From` is always in scope. `TryFrom` and `UnsafeFrom` are imported from `std::convert`:

```rust
use std::convert::{TryFrom, UnsafeFrom};

fn main(x: u32, y: u64) {
    let exact = Field::from(x);            // Every u32 is a field element
    let checked = Field::try_from(y);      // None if y is not a field element
    let wrapped = Field::unsafe_from(y);   // y modulo the field

    let byte = u8::try_from(exact);        // None if exact is 256 or more
    let low_byte = u8::unsafe_from(exact); // The low 8 bits of exact
}
```

A signed integer converts with `unsafe_from` as its bit pattern, so `-1_i8` becomes `255`, and `try_from` is `None` for a negative one.

### Performance note

Integer casts generate range check constraints in the compiled circuit. Using `Field` values directly where possible can avoid these extra constraints.

See [Security Considerations for Field Arithmetic](./fields.md#security-considerations-for-field-arithmetic) to determine if the tradeoffs are worth the cheaper casting & arithmetic costs of using field arithmetic directly.

## Type Coercions

When one type is required in Noir code but a different type is given, the compiler will typically issue
a type error. There are a few cases however where the compiler will instead automatically perform a
type coercion. These are typically limited to a few type pairs where converting from one to the other
will not sacrifice performance or correctness. Currently, Noir will will try to perform the following
type coercions:

| Actual Type    | Expected Type               |
| -------------- | --------------------------- |
| `[T; N]`       | `[T]`                       |
| `fn(..) -> R`  | `unconstrained fn(..) -> R` |
| `str<N>`       | `CtString`                  |
| `fmtstr<N, T>` | `CtString`                  |
| `&mut T`       | `&T`                        |

Note that:
- Conversions are only from the actual type to the expected type, never the other way around.
- Conversions are only performed on the outermost type, they're never performed within a nested type.
- `CtString` is a compile-time only type, so this conversion is only valid in [comptime code](../comptime.md).

Examples:
```rust
fn requires_vector(_vector: [Field]) {}
comptime fn requires_ct_string(_s: CtString) {}

fn main() {
    let array: [Field; 4] = [1, 2, 3, 4];

    // Ok - array is converted to a vector
    requires_vector(array);
    // equivalent to:
    requires_vector(array.as_vector());

    // coerce a constrained function to an unconstrained one:
    let f: unconstrained fn([Field]) = requires_vector;

    comptime {
        // Passing a str<6> where a CtString is expected
        requires_ct_string("hello!")
    }
}
```
