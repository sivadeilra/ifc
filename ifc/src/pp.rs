//! Preprocessing Forms - Chapter 18

use super::*;

tagged_index! {
    pub struct FormIndex {
        const TAG_BITS: usize = 4;
        tag: FormSort,
        index: u32,
    }
}

#[c_enum(storage = "u32")]
pub enum FormSort {
    IDENTIFIER = 0,
    NUMBER = 1,
    CHARACTER = 2,
    STRING = 3,
    OPERATOR = 4,
    KEYWORD = 5,
    WHITESPACE = 6,
    PARAMETER = 7,
    STRINGIZE = 8,
    CATENATE = 9,
    PRAGMA = 10,
    HEADER = 11,
    PARENTHESIZED = 12,
    TUPLE = 13,
    JUNK = 14,
}

/// C++ preprocessor macro definitions are indicated by macro abstract references.
#[c_enum(storage = "u32")]
pub enum MacroSort {
    OBJECT_LIKE = 0,
    FUNCTION_LIKE = 1,
}

#[repr(C)]
#[derive(AsBytes, FromBytes, Clone)]
pub struct MacroObjectLike {
    pub locus: SourceLocation,
    pub name: TextOffset,
    pub body: FormIndex,
}

#[repr(C)]
#[derive(AsBytes, FromBytes, Clone)]
pub struct MacroFunctionLike {
    pub locus: SourceLocation,
    pub name: TextOffset,
    pub parameters: FormIndex,
    pub body: FormIndex,
    pub arity_and_variadic: ArityAndVariadic,
}

impl MacroFunctionLike {
    pub fn arity(&self) -> u32 {
        self.arity_and_variadic.arity()
    }

    pub fn is_variadic(&self) -> bool {
        self.arity_and_variadic.is_variadic()
    }
}

#[derive(AsBytes, FromBytes, Clone, Copy, Eq, PartialEq)]
#[repr(transparent)]
pub struct ArityAndVariadic(pub u32);

impl ArityAndVariadic {
    pub fn arity(&self) -> u32 {
        self.0 & 0x7fff_ffff
    }

    pub fn is_variadic(&self) -> bool {
        (self.0 & 0x8000_000) != 0
    }
}

/// `pp.ident`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormIdentifier {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
}

/// `pp.char`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormCharacter {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
}

/// `pp.string`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormString {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
}

/// `pp.num`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormNumber {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
}

/// `pp.op`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormOperator {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
    pub operator: FormOp,
}

tagged_index! {
    pub struct FormOp {
        const TAG_BITS: usize = 3;
        tag: WordSort,
        index: u16,
    }
}

impl FormOp {
    pub fn value(self) -> PreProcessingOpOrPunc {
        match self.tag() {
            WordSort::PUNCTUATOR => PreProcessingOpOrPunc::Punctuator(WordSortPunctuator::from_u32(self.index())),
            WordSort::OPERATOR => PreProcessingOpOrPunc::Operator(WordSortOperator::from_u32(self.index())),
            _ => panic!("Not an operator or punctuator"),
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum PreProcessingOpOrPunc {
    Punctuator(WordSortPunctuator),
    Operator(WordSortOperator),
}


/// `pp.key`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormKeyword {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
}

/// `pp.space`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormWhitespace {
    pub locus: SourceLocation,
}


/// `pp.param`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormParameter {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
}


/// `pp.to-string`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormStringize {
    pub locus: SourceLocation,
    pub operand: FormIndex,
}

/// `pp.catenate`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormCatenate {
    pub locus: SourceLocation,
    pub first: FormIndex,
    pub second: FormIndex,
}

/// `pp.header`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormHeader {
    pub locus: SourceLocation,
    pub spelling: FormIndex,
}

/// `pp.paren`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormParen {
    pub locus: SourceLocation,
    pub operand: FormIndex,
}


/// `pp.tuple`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormTuple {
    pub start: Index,
    pub cardinality: Cardinality,
}

/// `pp.junk`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormJunk {
    pub locus: SourceLocation,
    pub spelling: TextOffset,
}

/// `pp.pragma`
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct FormPragma {
    pub locus: SourceLocation,
    pub operand: FormIndex,
}
