use super::*;
use bitflags::bitflags;

// Chapter 9 Types

// heap.type
tagged_index! {
    pub struct TypeIndex {
        const TAG_BITS: usize = 5;
        tag: TypeSort,
        index: u32,
    }
}

impl TypeIndex {
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

#[c_enum(storage = "u32")]
pub enum TypeSort {
    VENDOR_EXTENSION = 0x00,
    METHOD = 0x0B,
    FUNDAMENTAL = 0x01,
    ARRAY = 0x0C,
    DESIGNATED = 0x02,
    TYPENAME = 0x0D,
    DEDUCED = 0x03,
    QUALIFIED = 0x0E,
    SYNTACTIC = 0x04,
    BASE = 0x0F,
    EXPANSION = 0x05,
    DECLTYPE = 0x10,
    POINTER = 0x06,
    PLACEHOLDER = 0x11,
    POINTER_TO_MEMBER = 0x07,
    TUPLE = 0x12,
    LVALUE_REFERENCE = 0x08,
    FORALL = 0x13,
    RVALUE_REFERENCE = 0x09,
    UNALIGNED = 0x14,
    FUNCTION = 0x0A,
    SYNTAX_TREE = 0x15,
}

// 9.1.2.1 Fundamental type basis

// type.fundamental
#[repr(C)]
#[derive(AsBytes, FromBytes, Debug)]
pub struct FundamentalType {
    pub basis: TypeBasis,
    pub precision: TypePrecision,
    pub sign: TypeSign,
    pub padding: [u8; 1],
}

#[c_enum(storage = "u8")]
pub enum TypeBasis {
    VOID,
    BOOL,
    CHAR,
    WCHAR_T,
    INT,
    FLOAT,
    DOUBLE,
    NULLPTR,
    ELLIPSIS,
    SEGMENT_TYPE,
    CLASS,
    STRUCT,
    UNION,
    ENUM,
    TYPENAME,
    NAMESPACE,
    INTERFACE,
    FUNCTION,
    EMPTY,
    VARIABLE_TEMPLATE,
    AUTO,
    DECLTYPE_AUTO,
}

// 9.1.2.2 Fundamental type precision
// The bit precision of a funamental type is a value of type TypePrecision defined as
// follows:
#[c_enum(storage = "u8")]
pub enum TypePrecision {
    DEFAULT,
    SHORT,
    LONG,
    BIT8,
    BIT16,
    BIT32,
    BIT64,
    BIT128,
}

// The sign of a fundamental type is expressed as a value of type TypeSign defined as
// follows:
#[c_enum(storage = "u8")]
pub enum TypeSign {
    PLAIN,
    SIGNED,
    UNSIGNED,
}

/// `type.base`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct TypeBase {
    pub ty: TypeIndex,
    pub access: Access,
    pub specifiers: BaseTypeSpecifiers,
    pub __padding: [u8; 2],
}

#[c_enum(storage = "u8")]
pub enum BaseTypeSpecifiers {
    NONE = 0,
    SHARED = 1,
    EXPANDED = 2,
}

// 9.1.11
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FunctionType {
    /// Return type of the function type
    pub target: TypeIndex,
    /// Parameter type list. A null `source` value indicates no parameter type.
    /// If function type has at most one parameter type, the `source` denotes
    /// that type. Otherwise, it is a tuple type made of all of the parameter types.
    pub source: TypeIndex,
    pub eh_spec: NoexceptSpecification,
    pub convention: CallingConvention,
    pub traits: FunctionTypeTraits,
    pub padding: [u8; 2],
}

// 9.1.11.1
bitflags! {
    #[derive(AsBytes, FromBytes)]
    #[repr(transparent)]
    pub struct FunctionTypeTraits : u8 {
        const NONE = 0;
        const CONST_TRAIT = 1 << 0;
        const VOLATILE = 1 << 1;
        const LVALUE = 1 << 2;
        const RVALUE = 1 << 3;
    }
}

#[c_enum(storage = "u8")]
pub enum CallingConvention {
    Cdecl,
    Fast,
    Std,
    This,
    Clr,
    Vector,
    Eabi,
}

#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct NoexceptSpecification {
    pub words: SentenceIndex,
    pub sort: NoexceptSort,
    pub padding: [u8; 3],
}

// 9.1.15
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct QualifiedType {
    pub unqualified_type: TypeIndex,
    pub qualifiers: Qualifiers,
    pub padding: [u8; 3],
}

// 9.1.19
#[repr(C)]
#[derive(AsBytes, FromBytes)]
pub struct TupleType {
    // Index into the type heap partition.
    pub start: Index,
    pub cardinality: Cardinality,
}

bitflags! {
    #[derive(AsBytes, FromBytes)]
    #[repr(transparent)]
    pub struct Qualifiers : u8 {
        const NONE = 0;
        const CONST = 1 << 0;
        const VOLATILE = 1 << 1;
        const RESTRICT = 1 << 2;
    }
}

impl Ifc {
    pub fn is_void(&self, t: TypeIndex) -> bool {
        if t.tag() != TypeSort::FUNDAMENTAL {
            return false;
        }

        let tf = self.type_fundamental().entry(t.index()).unwrap();
        tf.basis == TypeBasis::VOID
    }

    pub fn as_fundamental_type(&self, t: TypeIndex) -> Option<&FundamentalType> {
        if t.tag() == TypeSort::FUNDAMENTAL {
            Some(self.type_fundamental().entry(t.index()).unwrap())
        } else {
            None
        }
    }
}

/// Partition `type.array`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct TypeArray {
    pub element: TypeIndex,
    pub extent: ExprIndex,
}
