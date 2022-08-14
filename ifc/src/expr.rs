//! Expressions - Chapter 10

use super::*;

tagged_index! {
    pub struct ExprIndex {
        const TAG_BITS: usize = 6;
        tag: ExprSort,
        index: u32,
    }
}

#[c_enum(storage = "u32")]
pub enum ExprSort {
    VENDOR_EXTENSION = 0,
    EMPTY = 1,
    LITERAL = 2,
    LAMBDA = 3,
    TYPE = 4,
    NAMED_DECL = 5,
    UNRESOLVED_ID = 6,
    TEMPLATE_ID = 6,
    UNQUALIFIED_ID = 8,
    SIMPLE_IDENTIFIER = 9,
    POINTER = 10,
    QUALIFIED_NAME = 11,
    PATH = 12,
    READ = 13,
    MONAD = 14,
    DYAD = 15,
    TRIAD = 16,
    STRING = 17,
    TEMPORARY = 18,
    CALL = 19,
    MEMBER_INITIALIZER = 20,
    MEMBER_ACCESS = 21,
    INHERITANCE_PATH = 22,
    INITIALIZER_LIST = 23,
    CAST = 24,
    CONDITION = 25,
    EXPRESSION_LIST = 26,
    SIZEOF_TYPE = 27,
    ALIGNOF = 28,
    NEW = 29,
    DELETE = 30,
    TYPEID = 31,
    DESTRUCTOR_CALL = 32,
    SYNTAX_TREE = 33,
}

/// Partition `expr.literal`

#[repr(C)]
#[derive(Clone, AsBytes, FromBytes, Debug)]
pub struct ExprLiteral {
    pub locus: SourceLocation,
    pub ty: TypeIndex,
    pub value: LitIndex,
}

tagged_index! {
    pub struct LitIndex {
        const TAG_BITS: usize = 2;
        tag: LiteralSort,
        index: u32,
    }
}


#[c_enum(storage = "u32")]
pub enum LiteralSort {
    /// The `value` field directly holds a 32-bit unsigned integer value.
    IMMEDIATE = 0,
    /// The `value` field is an index into the `const.i64` partition. The value at that entry is a
    /// 64-bit unsigned integer.
    INTEGER = 1,
    /// The `value` fiels is an iundex into the `const.f64` partition.
    FLOATING_POINT = 2,
}

/// Partition `const.f64`
#[repr(C)]
#[derive(Clone, AsBytes, FromBytes)]
pub struct ConstF64 {
    pub f64_bytes: [u8; 8],
    pub unspecified: [u8; 4],
}
