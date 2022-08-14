//! Section 19.2 - Words

use super::*;

/// 19.2 Words
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct Word {
    pub locus: SourceLocation,
    // TODO: The spec says that this has a 16-bit value, but it also says that the type of this
    // field is "Index", which is u32.
    pub index: Index,
    pub value: u16,
    pub sort: WordSort,
    pub __padding: u8,
}

#[c_enum(storage = "u8")]
pub enum WordSort {
    UNKNOWN = 0,
    DIRECTIVE = 1,
    PUNCTUATOR = 2,
    LITERAL = 3,
    OPERATOR = 4,
    KEYWORD = 5,
    IDENTIFIER = 6,
}
