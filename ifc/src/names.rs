//! Names - Chapter 12

use super::*;

/// `name.operator`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone)]
pub struct NameOperator {
    pub encoded: TextOffset,
    pub operator: u16,
    pub __padding: u16,
}

struct Operator {
}
