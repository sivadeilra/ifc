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

#[c_enum(storage = "u16")]
pub enum WordSortPunctuator {
    Unknown = 0x00,
    LeftParenthesis = 0x01,
    RightParenthesis = 0x02,
    LeftBracket = 0x03,
    RightBracket = 0x04,
    LeftBrace = 0x05,
    RightBrace = 0x06,
    Colon = 0x07,
    Question = 0x08,
    Semicolon = 0x09,
    ColonColon = 0x0A,
    Msvc = 0x1FFF,
    MsvcZeroWidthSpace = 0x2000,
    MsvcEndOfPhrase = 0x2001,
    MsvcFullStop = 0x2002,
    MsvcNestedTemplateStart = 0x2003,
    MsvcDefaultArgumentStart = 0x2004,
    MsvcAlignasEdictStart = 0x2005,
    MsvcDefaultInitStart = 0x2006,
}

#[c_enum(storage = "u16")]
pub enum WordSortOperator {
    Unknown = 0x00,
    Equal = 0x01,
    Comma = 0x02,
    Exclaim = 0x03,
    Plus = 0x04,
    Dash = 0x05,
    Star = 0x06,
    Slash = 0x07,
    Percent = 0x08,
    LeftChevron = 0x09,
    RightChevron = 0x0A,
    Tilde = 0x0B,
    Caret = 0x0C,
    Bar = 0x0D,
    Ampersand = 0x0E,
    PlusPlus = 0x0F,
    DashDash = 0x10,
    Less = 0x11,
    LessEqual = 0x12,
    Greater = 0x13,
    GreaterEqual = 0x14,
    EqualEqual = 0x15,
    ExclaimEqual = 0x16,
    Diamond = 0x17,
    PlusEqual = 0x18,
    DashEqual = 0x19,
    StarEqual = 0x1A,
    SlashEqual = 0x1B,
    PercentEqual = 0x1C,
    AmpersandEqual = 0x1D,
    BarEqual = 0x1E,
    CaretEqual = 0x1F,
    LeftChevronEqual = 0x20,
    RightChevronEqual = 0x21,
    AmpersandAmpersand = 0x22,
    BarBar = 0x23,
    Ellipsis = 0x24,
    Dot = 0x25,
    Arrow = 0x26,
    DotStar = 0x27,
    ArrowStar = 0x28,
}
