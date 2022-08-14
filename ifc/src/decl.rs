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
        // const CXX = 0;                              // C++ language linkage
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
bitflags! {
    #[derive(AsBytes, FromBytes)]
    #[repr(transparent)]
    pub struct ObjectTraits : u8 {
        const NONE = 0;
        const CONSTEXPR = 1;
        const MUTABLE = 2;
        const THREAD_LOCAL = 4;
        const INLINE = 8;
        const INITIALIZER_EXPORTED = 0x10;
        const VENDOR = 0x80;
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
    /// An ordinary textual identifier.  The index value is a TextOffset.
    IDENTIFIER = 0,

    /// An operator function name. The index field is an index into the operator partition
    /// (`name.operator`). Each entry in that partition has two components: a _category_ field
    /// denoting the specified operator, and an _encoded_ field.
    OPERATOR = 1,

    /// A conversion-function name. The _index_ is an index into the conversion function name
    /// partition (`name.conversion`).
    CONVERSION = 2,

    /// A reference to a string literal operator name. The _index_ field is an index into the
    /// string literal operator partition (`name.literal`).
    LITERAL = 3,

    /// A reference to an assumed (as opposed to _declared_) template name. This is the case of
    /// nested-name of qualified-id where the qualifier is a dependent name and the unqualified
    /// part is asserted to name a template.  The _index_ field is an index into the partition
    /// of assumed template names (`name.template`).
    TEMPLATE = 4,

    /// A reference to a template-id, i.e. what in C++ source code is a template-name followed by
    /// a template-argument list.  The _index_ field is an index into the template-id partition
    /// (`name.specialization`).
    SPECIALIZATION = 5,

    /// A reference to a source file name. The _index_ field is an index into the partition of
    /// source file names (`name.source-file`).
    SOURCE_FILE = 6,

    /// A reference to a user-authored deduction guide name for a class template. Note that
    /// deduction guides don't have names at the C++ source level. The _index_ field is an index
    /// into the deduction guides partition (`name.guide`).
    GUIDE = 7,
}

/// for `name.source-file`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct NameSourceFile {
    pub path: TextOffset,
    pub guard: TextOffset,
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


// 8.2
// "decl.scope"
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct DeclScope {
    pub name: NameIndex,
    pub locus: SourceLocation,

    pub ty: TypeIndex,

    pub base: TypeIndex,
    pub initializer: ScopeIndex,
    pub home_scope: DeclIndex,
    pub alignment: ExprIndex,
    pub pack_size: PackSize,
    pub specifiers: BasicSpecifiers,
    pub traits: ScopeTraits,
    pub access: Access,
    pub properties: ReachableProperties,
    pub __padding: [u8; 2],
}

#[c_enum(storage = "u8")]
pub enum ScopeTraits {
    NONE = 0,
    UNNAMED = 1,
    INLINE = 2,
    INITIALIZER_EXPORTED = 4,
    CLOSURE_TYPE = 8,
    FINAL = 0x40,
    VENDOR = 0x80,
}

pub type PackSize = u16;

#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct DeclField {
    pub name: TextOffset,
    pub locus: SourceLocation,
    pub ty: TypeIndex,
    pub home_scope: DeclIndex,
    pub initializer: ExprIndex,
    pub alignment: ExprIndex,
    pub traits: ObjectTraits,
    pub specifier: BasicSpecifiers,
    pub access: Access,
    pub properties: ReachableProperties,
}

#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct DeclEnum {
    pub name: TextOffset,
    pub locus: SourceLocation,
    pub ty: TypeIndex,
    pub base: TypeIndex,
    pub initializer: Sequence,
    pub home_scope: DeclIndex,
    pub alignment: ExprIndex,
    pub specifiers: BasicSpecifiers,
    pub access: Access,
    pub properties: ReachableProperties,
    pub __padding: [u8; 1],
}

#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct DeclEnumerator {
    pub name: TextOffset,
    pub locus: SourceLocation,
    pub ty: TypeIndex,
    pub initializer: ExprIndex,
    pub specifier: BasicSpecifiers,
    pub access: Access,
    pub __padding: [u8; 2],
}

