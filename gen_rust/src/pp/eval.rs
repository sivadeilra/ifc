#![allow(dead_code)]

#[derive(Copy, Clone, Eq, PartialEq)]
enum Op {
    Add,
    Mul,
    Sub,
    Neg,
    Paren,
}

struct EvalContext {
    pub items: Vec<Item>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Value {
    F32(f32),
    F64(f64),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
}

enum Item {
    /// top of stack is lparen for nested expression, but expression is empty
    LParen,
    /// top of stack is binary operator
    Op(Op),
    Value(Value),
}
