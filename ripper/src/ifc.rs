// https://microsoft-my.sharepoint.com/personal/ardavis_microsoft_com/Documents/Documents/ifc.pdf

use super::*;
use bitflags::bitflags;
use c_macros::c_enum;
use core::fmt::{Debug, Formatter};
use core::mem::size_of;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

pub mod decl;
pub mod parts;
pub mod types;

pub use decl::*;
pub use parts::*;
pub use types::*;

#[repr(C)]
#[derive(FromBytes, AsBytes, Clone, Default)]
pub struct Sha256 {
    pub bytes: [u8; 32],
}
impl core::fmt::Debug for Sha256 {
    fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
        for &b in self.bytes.iter() {
            write!(fmt, "{:02x}", b)?;
        }
        Ok(())
    }
}

pub type ByteOffset = u32;
pub type Cardinality = u32;
pub type EntitySize = u32;
pub type Index = u32;

struct Sequence {
    pub start: Index,
    pub cardinality: Cardinality,
}

pub type Version = u8;

pub type Abi = u8;

#[c_enum(storage = "u8")]
pub enum Architecture {
    Unknown = 0,
    X86 = 1,
    X64 = 2,
    ARM32 = 3,
    ARM64 = 4,
    HybridX86ARM64 = 5,
}

pub type LanguageVersion = u32;

pub const IFC_FILE_SIGNATURE: [u8; 4] = [0x54, 0x51, 0x45, 0x1A];

// This is never defined in the spec.
pub type Bool = u8;

#[repr(C)]
#[derive(FromBytes, AsBytes, Clone, Default, Debug)]
pub struct FileHeader {
    pub checksum: Sha256,
    pub major_version: Version,
    pub minor_version: Version,
    pub abi: Abi,
    pub arch: Architecture,
    pub dialect: LanguageVersion,
    pub string_table_bytes: ByteOffset,
    pub string_table_size: Cardinality,
    pub unit: UnitIndex,
    pub src_path: TextOffset,
    pub global_scope: ScopeIndex,
    pub toc: ByteOffset,
    pub partition_count: Cardinality,
    pub internal: Bool,
    pub padding: [u8; 3],
}

// chapter 3

pub type TextOffset = u32;

#[c_enum]
pub enum UnitSort {
    SOURCE = 0,
    PRIMARY = 1,
    PARTITION = 2,
    HEADER = 3,
    EXPORTED_TU = 4,
}

// chapter 4

// bits 0-2 are UnitSort
// bits 3-31 are index
pub type UnitIndex = u32;

// chapter 6

pub type ScopeIndex = u32;

// 6.2
// contained in scope.desc
#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct ScopeDescriptor {
    // index into the scope member partition (see 6.3) (`scope.member`)
    pub start: Index,
    pub cardinality: Cardinality,
}

// 7 Heaps

pub type StmtIndex = u32; // heap.stmt
pub type ExprIndex = u32; // heap.expr
pub type SyntaxIndex = u32; // heap.syn
pub type FormIndex = u32; // heap.form
pub type ChartIndex = u32; // heap.chart

// Chapter 15

pub type LineIndex = u32;
pub type Column = u32;

#[repr(C)]
#[derive(AsBytes, FromBytes, Clone, Debug)]
pub struct SourceLocation {
    pub line: LineIndex,
    pub column: Column,
}

// Chapter 17 Token Streams

pub type SentenceIndex = u32;
