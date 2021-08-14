use super::*;

// 8 Declarations

tagged_index! {
    pub struct DeclIndex {
        const TAG_BITS: usize = 5;
        tag: DeclSort,
        index: u32,
    }
}

#[c_enum(storage = "u32")]
pub enum DeclSort {
    VENDOR_EXTENSION = 0x00,
    METHOD = 0x10,
    ENUMERATOR = 0x01,
    CONSTRUCTOR = 0x11,
    VARIABLE = 0x02,
    INHERITED_CONSTRUCTOR = 0x12,
    PARAMETER = 0x03,
    DESTRUCTOR = 0x13,
    FIELD = 0x04,
    REFERENCE = 0x14,
    BITFIELD = 0x05,
    USING_DECLARATION = 0x15,
    SCOPE = 0x06,
    USING_DIRECTIVE = 0x16,
    ENUMERATION = 0x07,
    FRIEND = 0x17,
    ALIAS = 0x08,
    EXPANSION = 0x18,
    TEMPLOID = 0x09,
    DEDUCTION_GUIDE = 0x19,
    TEMPLATE = 0x0A,
    BARREN = 0x1A,
    PARTIAL_SPECIALIZATION = 0x0B,
    TUPLE = 0x1B,
    EXPLICIT_SPECIALIZATION = 0x0C,
    SYNTAX_TREE = 0x1C,
    EXPLICIT_INSTANTIATION = 0x0D,
    INTRINSIC = 0x1D,
    CONCEPT = 0x0E,
    PROPERTY = 0x1E,
    FUNCTION = 0x0F,
    OUTPUT_SEGMENT = 0x1F,
}

// 8.1 Declaration vocabulary types
#[c_enum(storage = "u8")]
pub enum Access {
    NONE = 0,
    PRIVATE = 1,
    PROTECTED = 2,
    PUBLIC = 3,
}

bitflags! {
    #[derive(AsBytes, FromBytes)]
    #[repr(transparent)]
    pub struct BasicSpecifiers : u8 {
        const CXX = 0;                              // C++ language linkage
        const C = 1 << 0;                           // C language linkage
        const INTERNAL = 1 << 1;                    //
        const VAGUE = 1 << 2;                       // Vague linkage, e.g. COMDAT, still external
        const EXTERNAL = 1 << 3;                    // External linkage.
        const DEPRECATED = 1 << 4;                  // [[deprecated("foo")]]
        const INITIALIZED_IN_CLASS = 1 << 5;        // defined or initialized in a class
        const NON_EXPORTED = 1 << 6;                // Not explicitly exported
        const IS_MEMBER_OF_GLOBAL_MODULE = 1 << 7;  // member of the global module
    }
}
// "decl.alias"
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct DeclAlias {
    pub name: TextOffset,            // 0
    pub locus: SourceLocation,       // 4
    pub type_: TypeIndex,            // 12
    pub home_scope: DeclIndex,       // 16
    pub aliasee: TypeIndex,          // 20
    pub specifiers: BasicSpecifiers, // 24
    pub access: Access,              // 25
    pub __padding: [u8; 2],          // 26
                                     // 28 sizeof
}

// 8.1.3

#[c_enum(storage = "u8")]
pub enum ReachableProperties {
    NONE = 0,                   // nothing beyond name, type, scope.
    INITIALIZER = 1 << 0,       // IPR-initializer exported.
    DEFAULT_ARGUMENTS = 1 << 1, // function or template default arguments exported
    ATTRIBUTES = 1 << 2,        // standard attributes exported.
    ALL = 0xff,                 // Everything.
}

#[c_enum(storage = "u32")]
pub enum NameSort {
    IDENTIFIER = 0,
    OPERATOR = 1,
    CONVERSION = 2,
    LITERAL = 3,
    TEMPLATE = 4,
    SPECIALIZATION = 5,
    SOURCE_FILE = 6,
    GUIDE = 7,
}

// Chapter 12
tagged_index! {
    pub struct NameIndex {
        const TAG_BITS: usize = 3;
        tag: NameSort,
        index: u32,
    }
}

// 8.2.16
// "decl.function"
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct DeclFunc {
    pub name: NameIndex,                 // 0
    pub locus: SourceLocation,           // 4
    pub type_: TypeIndex,                // 12
    pub home_scope: DeclIndex,           // 16
    pub chart: ChartIndex,               // 20
    pub traits: FunctionTraits,          // 24
    pub specifiers: BasicSpecifiers,     // 26
    pub access: Access,                  // 27
    pub properties: ReachableProperties, // 28
    pub padding: [u8; 3],                // 29
}

bitflags! {
    #[derive(FromBytes, AsBytes)]
    #[repr(transparent)]
    pub struct FunctionTraits : u16 {
        const NONE = 0;
        const INLINE = 1 << 0;
        const CONSTEXPR = 1 << 1;
        const EXPLICIT = 1 << 2;
        const VIRTUAL = 1 << 3;
        const NO_RETURN = 1 << 4;
        const PURE_VIRTUAL = 1 << 5;
        const HIDDEN_FRIEND = 1 << 6;
        const DEFAULTED = 1 << 7;
        const DELETED = 1 << 8;
        const CONSTRAINED = 1 << 9;
        const IMMEDIATE = 1 << 10;
        const VENDOR = 1 << 15;
        const ALL = 0xffff;
    }
}

// 8.23
#[c_enum(storage = "u8")]
pub enum NoexceptSort {
    NONE,
    FALSE,
    TRUE,
    EXPRESSION,
    INFERRED,
    UNENFORCED,
}
