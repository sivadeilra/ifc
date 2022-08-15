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

/// Partition `expr.dyad`
#[repr(C)]
#[derive(Clone, AsBytes, FromBytes, Debug)]
pub struct ExprDyad {
    locus: SourceLocation,
    ty: TypeIndex,
    impl_: DeclIndex,
    arguments: [ExprIndex; 2],
    assoc: DyadicOperator,
}


#[c_enum(storage = "u32")]
pub enum DyadicOperator {
    UNKNOWN = 0,
    PLUS = 1,
    MINUS = 2,
    MULT = 3,
    SLASH = 4,
    MODULO = 5,
    REMAINDER = 6,
    BITAND = 7,
    BITOR = 8,
    BITXOR = 9,
    LSHIFT = 10,
    RSHIFT = 11,
    EQUAL = 12,
    NOT_EQUAL = 13,
    LESS = 14,
    LESS_EQUAL = 15,
    GREATER = 16,
    GREATER_EQUAL = 17,
    COMPARE = 0x12,
    LOGIC_AND = 0x13,
    LOGIC_OR = 0x14,
    ASSIGN = 0x15,
    PLUS_ASSIGN = 0x16,
    MINUS_ASSIGN = 0x17,
    MULT_ASSIGN = 0x18,
    SLASH_ASSIGN = 0x19,
    MODULO_ASSIGN = 0x1A,
    BITAND_ASSIGN = 0x1b,
    BITOR_ASSIGN = 0x1c,
    BITXOR_ASSIGN = 0x1d,
    LSHIFT_ASSIGN = 0x1e,
    RSHIFT_ASSIGN = 0x1f,
    COMMA = 0x20,
    DOT = 0x21,
    ARROW = 0x22,
    DOT_STAR = 0x23,
    ARROW_STAR = 0x24,
    CURRY = 0x25,
    APPLY = 0x26,
    INDEX = 0x27,
    DEFAULT_AT = 0x28,
    NEW = 0x29,
    NEW_ARRAY = 0x2a,
    DESTRUCT = 0x2b,
    DESTRUCT_AT = 0x2c,
    CLEANUP = 0x2d,
    QUALIFICATION = 0x2d,
    PROMOTE = 0x2f,
    DEMOTE = 0x30,
    COERCE = 0x31,
    REWRITE = 0x32,
    BLESS = 0x33,
    CAST = 0x34,
    EXPLICIT_CONVERSION = 0x35,
    REINTERPRET_CAST = 0x36,
    STATIC_CAST = 0x37,
    CONST_CAST = 0x38,
    DYNAMIC_CAST = 0x39,
    NARROW = 0x3a,
    WIDEN = 0x3b,
    PRETEND = 0x3c,
    CLOSURE = 0x3d,
    ZERO_INITIALIZE = 0x3e,
    CLEAR_STORAGE = 0x3f,
    SELECT = 0x400,
    MSVC = 0x400,
    MSVC_TRY_CAST = 0x401,
    MSVC_CURRY = 0x402,
    MSVC_VIRTUAL_CURRY = 0x403,
    MSVC_ALIGN = 0x404,
    MSVC_BIT_SPAN = 0x405,
    MSVC_BITFIELD_ACCESS = 0x406,
    MSVC_OBSCURE_BITFIELD_ACCESS = 0x407,
    MSVC_INITIALIZE = 0x408,
    MSVC_BUILTIN_OFFSET_OF = 0x409,
    MSVC_IS_BASE_OF = 0x40a,
    MSVC_IS_CONVERTIBLE_TO = 0x40b,
    MSVC_IS_TRIVIALLY_ASSIGNABLE = 0x40c,
    MSVC_IS_NOTHROW_ASSIGNABLE = 0x40d,
    MSVC_IS_ASSIGNABLE = 0x40e,
    MSVC_IS_ASSIGNABLE_NOCHECK = 0x40f,
    MSVC_BUILTIN_BITCAST = 0x410,
    MSVC_BUILTIN_IS_LAYOUT_COMPATIBLE = 0x411,
    MSVC_BUILTIN_IS_POINTER_INTERCONVERTIBLE_BASE_OF = 0x412,
    MSVC_BUILTIN_IS_POINTER_INTERCONVERTIBLE_WITH_CLASS = 0x413,
    MSVC_BUILTIN_IS_CORRESPONDING_MEMBER = 0x414,
    MSVC_INTRINSIC = 0x415,
    MSVC_SATURATED_ARITHMETIC = 0x416,
}
