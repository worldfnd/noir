use super::{IResult, InterpreterError, Value};
use crate::ast::BinaryOpKind;
use crate::hir::Location;
use crate::hir::comptime::Integer;
use crate::hir_def::expr::HirBinaryOp;

pub(super) fn evaluate_infix(
    lhs_value: Value,
    rhs_value: Value,
    operator: HirBinaryOp,
    location: Location,
) -> IResult<Value> {
    use BinaryOpKind::*;

    let lhs_type = lhs_value.get_type().into_owned();
    let rhs_type = rhs_value.get_type().into_owned();
    let symbol = operator.kind.as_str();

    let error = || {
        let lhs = lhs_type.clone();
        let rhs = rhs_type.clone();
        InterpreterError::InvalidValuesForBinary { lhs, rhs, location, operator: symbol }
    };
    let overflow = |operator| InterpreterError::BinaryOperationOverflow { location, operator };

    if matches!(operator.kind, Divide | Modulo)
        && let Value::Integer(rhs_value) = &rhs_value
        && rhs_value.is_zero()
    {
        return Err(error());
    }

    match (lhs_value, rhs_value) {
        (Value::Integer(lhs), Value::Integer(rhs)) => {
            if lhs.signed_and_bits() != rhs.signed_and_bits() {
                return Err(error());
            }
            let is_field = matches!(lhs, Integer::Field(_));
            let checked = |result: Option<Integer>| {
                result.map(Value::Integer).ok_or_else(|| overflow(symbol))
            };
            match operator.kind {
                Add => checked(lhs + rhs),
                Subtract => checked(lhs - rhs),
                Multiply => checked(lhs * rhs),
                Divide => checked(lhs / rhs),
                Modulo if is_field => Err(error()),
                Modulo => checked(lhs % rhs),
                Equal => Ok(Value::Bool(lhs == rhs)),
                NotEqual => Ok(Value::Bool(lhs != rhs)),
                Less => lhs.lt(&rhs).map(Value::Bool).ok_or_else(error),
                LessEqual => lhs.lte(&rhs).map(Value::Bool).ok_or_else(error),
                Greater => rhs.lt(&lhs).map(Value::Bool).ok_or_else(error),
                GreaterEqual => rhs.lte(&lhs).map(Value::Bool).ok_or_else(error),
                And => (lhs & rhs).map(Value::Integer).ok_or_else(error),
                Or => (lhs | rhs).map(Value::Integer).ok_or_else(error),
                Xor => (lhs ^ rhs).map(Value::Integer).ok_or_else(error),
                ShiftLeft | ShiftRight if is_field => Err(error()),
                ShiftLeft | ShiftRight => {
                    // An amount that is not a `u32` is reported as a `>>` overflow for either direction.
                    let amount = u32::try_from(&rhs.to_bigint()).map_err(|_| overflow(">>"))?;
                    let shifted = if operator.kind == ShiftLeft {
                        lhs.checked_shl(amount)
                    } else {
                        lhs.checked_shr(amount)
                    };
                    checked(shifted)
                }
            }
        }
        (Value::Bool(lhs), Value::Bool(rhs)) => match operator.kind {
            Equal => Ok(Value::Bool(lhs == rhs)),
            NotEqual => Ok(Value::Bool(lhs != rhs)),
            Less => Ok(Value::Bool(!lhs & rhs)),
            LessEqual => Ok(Value::Bool(lhs <= rhs)),
            Greater => Ok(Value::Bool(lhs & !rhs)),
            GreaterEqual => Ok(Value::Bool(lhs >= rhs)),
            And => Ok(Value::Bool(lhs & rhs)),
            Or => Ok(Value::Bool(lhs | rhs)),
            Xor => Ok(Value::Bool(lhs ^ rhs)),
            Add | Subtract | Multiply | Divide | Modulo | ShiftLeft | ShiftRight => Err(error()),
        },
        _ => Err(error()),
    }
}

#[cfg(test)]
mod tests {
    use crate::hir::comptime::InterpreterError;
    use crate::hir::comptime::tests::{interpret, interpret_expect_error};

    use super::{BinaryOpKind, HirBinaryOp, Location, Value};

    use super::evaluate_infix;

    #[test]
    /// See: <https://github.com/noir-lang/noir/issues/8391>
    fn regression_8391() {
        let lhs = Value::u128(340282366920938463463374607431768211455);
        let rhs = Value::u128(2);
        let operator = HirBinaryOp { kind: BinaryOpKind::Divide, location: Location::dummy() };
        let location = Location::dummy();
        let result = evaluate_infix(lhs, rhs, operator, location).unwrap();

        assert_eq!(result, Value::u128(170141183460469231731687303715884105727));
    }

    #[test]
    fn field_ordering_is_rejected() {
        use acvm::{FieldId, FieldValue};

        let neg_one = Value::field(-FieldValue::one(FieldId::linked()));
        let zero = Value::field(FieldValue::zero(FieldId::linked()));

        for kind in [
            BinaryOpKind::Less,
            BinaryOpKind::LessEqual,
            BinaryOpKind::Greater,
            BinaryOpKind::GreaterEqual,
        ] {
            let operator = HirBinaryOp { kind, location: Location::dummy() };
            let err = evaluate_infix(neg_one.clone(), zero.clone(), operator, Location::dummy())
                .unwrap_err();
            assert!(
                matches!(err, InterpreterError::InvalidValuesForBinary { .. }),
                "expected {kind:?} on fields to be rejected, got {err:?}"
            );
        }
    }

    #[test]
    fn field_equality() {
        use acvm::{FieldId, FieldValue};

        let one = Value::field(FieldValue::one(FieldId::linked()));
        let other_one = Value::field(FieldValue::one(FieldId::linked()));

        let operator = HirBinaryOp { kind: BinaryOpKind::Equal, location: Location::dummy() };
        let result = evaluate_infix(one, other_one, operator, Location::dummy()).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn regression_9336() {
        let lhs = Value::i8(-128);
        let rhs = Value::i8(-1);
        let operator = HirBinaryOp { kind: BinaryOpKind::Modulo, location: Location::dummy() };
        let location = Location::dummy();
        let err = evaluate_infix(lhs, rhs, operator, location).unwrap_err();
        assert!(matches!(err, InterpreterError::BinaryOperationOverflow { .. }));
    }

    #[test]
    fn shl_unsigned() {
        let src = r#"
            comptime fn main() -> pub u64 {
                3 << 4
            }
        "#;
        let result = interpret(src);
        assert_eq!(result, Value::u64(48));
    }

    #[test]
    fn shl_signed() {
        let src = r#"
            comptime fn main() -> pub i64 {
                2 << 3
            }
        "#;
        let result = interpret(src);
        assert_eq!(result, Value::i64(16));
    }

    #[test]
    fn shl_unsigned_overflow() {
        let src = r#"
            comptime fn main() -> pub u64 {
                1 << 128
            }
        "#;

        let err = interpret_expect_error(src);
        let InterpreterError::BinaryOperationOverflow { operator, .. } = err else {
            panic!("Expected overflow error");
        };
        assert_eq!(operator, "<<");
    }

    #[test]
    fn shl_signed_overflow() {
        let src = r#"
            comptime fn main() -> pub i64 {
                1 << 64
            }
        "#;

        let err = interpret_expect_error(src);
        let InterpreterError::BinaryOperationOverflow { operator, .. } = err else {
            panic!("Expected overflow error");
        };
        assert_eq!(operator, "<<");
    }

    #[test]
    fn shr_unsigned() {
        let src = r#"
            comptime fn main() -> pub u64 {
                64 >> 1
            }
        "#;
        let result = interpret(src);
        assert_eq!(result, Value::u64(32));
    }

    #[test]
    fn shr_unsigned_overflow() {
        let src = r#"
            comptime fn main() -> pub u64 {
                64 >> 63
            }
        "#;
        let result = interpret(src);
        assert_eq!(result, Value::u64(0));

        let src = r#"
            comptime fn main() -> pub u64 {
                64 >> 255
            }
        "#;
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));

        let src = "
            comptime fn main() -> pub u32 {
                1360887544 >> 141
            }
        ";
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));
    }

    #[test]
    fn shr_signed_overflow_negative_lhs() {
        let src = "
        comptime fn main() -> pub i64 {
            -64 >> 63
        }
        ";
        let result = interpret(src);
        assert_eq!(result, Value::i64(-1));

        let src = "
        comptime fn main() -> pub i64 {
            -64 >> 255
        }
        ";
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));

        let src = "
        comptime fn main() -> pub i32 {
            -1360887544 >> 141
        }
        ";
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));
    }

    #[test]
    fn shr_signed_overflow_positive_lhs() {
        let src = "
        comptime fn main() -> pub i64 {
            64 >> 63
        }
        ";
        let result = interpret(src);
        assert_eq!(result, Value::i64(0));

        let src = "
        comptime fn main() -> pub i64 {
            64 >> 255
        }
        ";
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));

        let src = "
        comptime fn main() -> pub i32 {
            1360887544 >> 141
        }
        ";
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));
    }

    #[test]
    fn shr_signed() {
        let src = r#"
            comptime fn main() -> pub i64 {
                -64 >> 1
            }
        "#;
        let result = interpret(src);
        assert_eq!(result, Value::i64(-32));
    }

    #[test]
    fn div_zero_field() {
        let src = r#"
            comptime fn main() -> pub Field {
                32 / 0
            }
        "#;
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::InvalidValuesForBinary { operator: "/", .. }));
    }

    #[test]
    fn div_zero_int() {
        let src = r#"
            comptime fn main() -> pub i32 {
                32 / 0
            }
        "#;
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::InvalidValuesForBinary { operator: "/", .. }));
    }

    #[test]
    fn div() {
        let src = r#"
            comptime fn main() {
                let x_field = 8;
                let x_i32: i32 = -7;

                assert_eq(x_field / 2, 4);
                assert_eq(x_i32 / 2, -3);
            }
        "#;
        let result = interpret(src);
        assert_eq!(result, Value::Unit);
    }

    #[test]
    fn shift_right_by_negative_number() {
        let src = r#"
            comptime fn main() {
                let _ = 1 >> -4294967296_i64;
            }
        "#;
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));
    }

    #[test]
    fn shift_left_by_negative_number() {
        let src = r#"
            comptime fn main() {
                let _ = 1 << -4294967296_i64;
            }
        "#;
        let result = interpret_expect_error(src);
        assert!(matches!(result, InterpreterError::BinaryOperationOverflow { operator: ">>", .. }));
    }

    #[test]
    fn binary_type_mismatch_is_reported() {
        for kind in [
            BinaryOpKind::Add,
            BinaryOpKind::Equal,
            BinaryOpKind::Less,
            BinaryOpKind::And,
            BinaryOpKind::ShiftLeft,
            BinaryOpKind::Modulo,
        ] {
            let operator = HirBinaryOp { kind, location: Location::dummy() };
            let err = evaluate_infix(Value::u8(1), Value::u16(1), operator, Location::dummy())
                .unwrap_err();
            assert!(
                matches!(err, InterpreterError::InvalidValuesForBinary { .. }),
                "{kind:?}: {err:?}"
            );
        }
    }

    #[test]
    fn field_integer_operations_are_rejected() {
        use acvm::{FieldId, FieldValue};

        let one = Value::field(FieldValue::one(FieldId::linked()));
        for kind in [
            BinaryOpKind::And,
            BinaryOpKind::Or,
            BinaryOpKind::Xor,
            BinaryOpKind::ShiftLeft,
            BinaryOpKind::ShiftRight,
            BinaryOpKind::Modulo,
        ] {
            let operator = HirBinaryOp { kind, location: Location::dummy() };
            let err =
                evaluate_infix(one.clone(), one.clone(), operator, Location::dummy()).unwrap_err();
            assert!(
                matches!(err, InterpreterError::InvalidValuesForBinary { .. }),
                "{kind:?}: {err:?}"
            );
        }
    }

    #[test]
    fn left_shift_truncates_to_width() {
        let shl = |lhs, rhs| {
            let operator =
                HirBinaryOp { kind: BinaryOpKind::ShiftLeft, location: Location::dummy() };
            evaluate_infix(lhs, rhs, operator, Location::dummy())
        };
        assert_eq!(shl(Value::u8(255), Value::u8(1)), Ok(Value::u8(254)));
        assert_eq!(shl(Value::i8(1), Value::i8(7)), Ok(Value::i8(-128)));
        assert!(matches!(
            shl(Value::u8(1), Value::u8(8)),
            Err(InterpreterError::BinaryOperationOverflow { operator: "<<", .. })
        ));
    }
}
