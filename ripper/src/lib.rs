// https://microsoft-my.sharepoint.com/personal/ardavis_microsoft_com/Documents/Documents/ifc.pdf

#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

use core::mem::size_of;
use core::ops::Range;
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

#[macro_use]
mod macros;

mod decl;
mod error;
mod parts;
mod types;
mod zerocopy_utils;

use bitflags::bitflags;
use c_macros::c_enum;
use core::fmt::{Debug, Formatter};
use error::*;
use zerocopy_utils::*;

pub use decl::*;
pub use error::*;
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

fn read_struct_at<T: AsBytes + FromBytes>(s: &[u8]) -> Result<T> {
    if core::mem::size_of::<T>() > s.len() {
        return Err(Error::bad("structure is larger than input slice"));
    }
    let mut value: T = unsafe { core::mem::zeroed::<T>() };
    value.as_bytes_mut().copy_from_slice(&s[..size_of::<T>()]);
    Ok(value)
}

fn get_slice(s: &[u8], range: Range<usize>) -> Result<&[u8]> {
    if let Some(rs) = s.get(range) {
        Ok(rs)
    } else {
        Err(Error::bad("range is out of bounds of input slice"))
    }
}

struct StringTable<'a> {
    strings: &'a [u8],
}

impl<'a> StringTable<'a> {
    fn get_string(&self, text_offset: TextOffset) -> Result<&'a str> {
        let offset = text_offset as usize;
        if let Some(strings_at_offset) = self.strings.get(offset..) {
            for (i, &b) in strings_at_offset.iter().enumerate() {
                if b == 0 {
                    let sb = &strings_at_offset[..i];
                    return match core::str::from_utf8(sb) {
                        Ok(s) => Ok(s),
                        Err(_) => Err(Error::BadString),
                    };
                }
            }
            // Never found end of string!
            Err(Error::bad(
                "string at end of string table was not NUL-terminated",
            ))
        } else {
            Err(Error::bad(
                "string offset was beyond bounds of string table",
            ))
        }
    }
}

pub struct Ifc {
    data: Vec<u8>,
    file_header: FileHeader,
    strings_range: Range<usize>,

    parts_map: HashMap<String, PartEntry>,
    parts: Parts,
}

impl Ifc {
    pub fn load(data: Vec<u8>) -> Result<Self> {
        let fs = data.as_slice();

        let sig = read_struct_at::<[u8; 4]>(&fs[0..])?;
        if sig != IFC_FILE_SIGNATURE {
            println!("File does not have IFC file signature.");
            return Err(Error::bad("File does not have IFC signature"));
        }

        let file_header = read_struct_at::<FileHeader>(&fs[4..])?;
        println!("File header: {:#?}", file_header);

        let strings_range = file_header.string_table_bytes as usize
            ..file_header.string_table_bytes as usize + file_header.string_table_size as usize;
        if fs.get(strings_range.clone()).is_none() {
            return Err(Error::bad("string table range is not valid"));
        }

        let mut ifc = Ifc {
            data,
            file_header,
            parts_map: HashMap::new(),
            strings_range,
            parts: Parts::default(),
        };

        let strings = StringTable {
            strings: &ifc.data[ifc.strings_range.clone()],
        };

        let num_partitions = ifc.file_header.partition_count;
        for i in 0..num_partitions as usize {
            let partition_summary = read_struct_at::<PartitionSummary>(
                &ifc.data[ifc.file_header.toc as usize + i * size_of::<PartitionSummary>()..],
            )?;

            let partition_name = strings.get_string(partition_summary.name)?;
            if false {
                println!(
                    "partition {}: {:-20} {:?}",
                    i, partition_name, partition_summary
                );
            }

            let part_range = partition_summary.offset as usize
                ..partition_summary.offset as usize
                    + partition_summary.cardinality as usize
                        * partition_summary.entity_size as usize;
            let part_data = if let Some(part_data) = ifc.data.get(part_range.clone()) {
                part_data
            } else {
                return Err(Error::bad_string(format!(
                    "partition {} {:?} is invalid; its range is outside the ifc file size",
                    i, partition_name
                )));
            };

            ifc.parts.load_part_data(
                partition_name,
                part_data,
                partition_summary.cardinality as usize,
                partition_summary.entity_size as usize,
            )?;

            ifc.parts_map.insert(
                partition_name.to_string(),
                PartEntry {
                    part_range,
                    count: partition_summary.cardinality as usize,
                    size: partition_summary.entity_size as usize,
                },
            );
        }

        Ok(ifc)
    }

    pub fn global_scope(&self) -> ScopeIndex {
        self.file_header.global_scope
    }

    pub fn parts(&self) -> &HashMap<String, PartEntry> {
        &self.parts_map
    }

    pub fn get_string(&self, text_offset: TextOffset) -> Result<&str> {
        StringTable {
            strings: &self.data[self.strings_range.clone()],
        }
        .get_string(text_offset)
    }

    /*
    pub fn get_part_by_name_opt<'a, 'p>(&'a self, name: &'p str) -> Option<Part<'a>> {
        match self.parts_map.get(name) {
            Some(part_entry) => Some(Part {
                part_name: name,
                part_data: &self.data[part_entry.part_range.clone()],
                count: part_entry.count,
                size: part_entry.size,
            }),
            None => None,
        }
    }

    pub fn get_part_by_name<'a, 'p>(&'a self, name: &'p str) -> Result<Part<'a, 'p>> {
        match self.parts_map.get(name) {
            Some(part_entry) => Ok(Part {
                part_name: name,
                part_data: &self.data[part_entry.part_range.clone()],
                count: part_entry.count,
                size: part_entry.size,
            }),
            None => Err(Error::MissingExpected),
        }
    }

    pub fn get_part_entry(&self, part_name: &str, index: u32) -> Result<&[u8]> {
        let part = self.get_part_by_name(part_name)?;
        let entry_start = part.size as usize * index as usize;
        if let Some(entry_bytes) = part
            .part_data
            .get(entry_start..entry_start + part.size as usize)
        {
            Ok(entry_bytes)
        } else {
            Err(Error::bad_string(format!(
                "index {} is out of range for partition '{}'",
                index, part_name
            )))
        }
    }
    */

    pub fn get_scope_descriptor(&self, scope_index: ScopeIndex) -> Result<&ScopeDescriptor> {
        self.scope_desc().entry(scope_index)
    }

    pub fn type_heap_lookup(&self, index: Index) -> Result<TypeIndex> {
        Ok(*self.heap_type().entry(index)?)
    }

    pub fn get_type_string(&self, type_index: TypeIndex) -> Result<String> {
        use core::fmt::Write;
        Ok(match type_index.tag() {
            TypeSort::FUNCTION => {
                let type_function_part = self.type_function();
                let type_func: &FunctionType = type_function_part.entry(type_index.index())?;

                let source_type_str = self.get_type_string(type_func.source)?;
                let target_type_str = self.get_type_string(type_func.target)?;
                let mut s = format!("return({}) args({})", target_type_str, source_type_str);
                let noexcept_str = match type_func.eh_spec.sort {
                    NoexceptSort::NONE => "",
                    NoexceptSort::FALSE => "noexcept(false)",
                    NoexceptSort::TRUE => "noexcept",
                    NoexceptSort::EXPRESSION => "noexcept(expr)",
                    NoexceptSort::INFERRED => "noexcept(inferred)",
                    NoexceptSort::UNENFORCED => "noexcept(unenforced)",
                    _ => "??",
                };
                s.push_str(" ");
                s.push_str(noexcept_str);
                s
            }

            TypeSort::FUNDAMENTAL => {
                let type_fundamental = self.type_fundamental().entry(type_index.index())?;
                // format!("{:?}", type_fundamental)
                match type_fundamental.basis {
                    TypeBasis::VOID => "void",
                    TypeBasis::BOOL => "bool",
                    TypeBasis::CHAR => "char",
                    TypeBasis::WCHAR_T => "wchar_t",
                    TypeBasis::INT => "int",
                    TypeBasis::FLOAT => "float",
                    TypeBasis::DOUBLE => "double",
                    TypeBasis::NULLPTR => "nullptr",
                    TypeBasis::ELLIPSIS => "ellipsis",
                    TypeBasis::SEGMENT_TYPE => "segment_type",
                    TypeBasis::CLASS => "class",
                    TypeBasis::STRUCT => "struct",
                    TypeBasis::UNION => "union",
                    TypeBasis::ENUM => "enum",
                    TypeBasis::TYPENAME => "typename",
                    TypeBasis::NAMESPACE => "namespace",
                    TypeBasis::INTERFACE => "interface",
                    TypeBasis::FUNCTION => "function",
                    TypeBasis::EMPTY => "empty",
                    TypeBasis::VARIABLE_TEMPLATE => "variable_template",
                    TypeBasis::AUTO => "auto",
                    TypeBasis::DECLTYPE_AUTO => "decltype_auto",
                    _ => "??",
                }
                .to_string()
            }

            TypeSort::TUPLE => {
                let type_tuple = self.type_tuple().entry(type_index.index())?;
                let mut s = String::new();
                s.push_str("(");
                for i in 0..type_tuple.cardinality {
                    if i > 0 {
                        s.push_str(", ");
                    }
                    let element_type_index = self.type_heap_lookup(type_tuple.start + i)?;
                    s.push_str(&self.get_type_string(element_type_index)?);
                }
                s.push_str(")");
                s
            }

            TypeSort::QUALIFIED => {
                let qualified_type: &QualifiedType =
                    self.type_qualified().entry(type_index.index())?;
                let mut s = self.get_type_string(qualified_type.unqualified_type)?;
                if qualified_type.qualifiers.contains(Qualifiers::CONST) {
                    s.insert_str(0, "const ");
                }
                if qualified_type.qualifiers.contains(Qualifiers::VOLATILE) {
                    s.insert_str(0, "volatile ");
                }
                s
            }

            TypeSort::POINTER => {
                let pointee_type: TypeIndex = *self.type_pointer().entry(type_index.index())?;
                let mut pointee_type_str = self.get_type_string(pointee_type)?;
                pointee_type_str.push_str("*");
                pointee_type_str
            }

            _ => format!("{:?}", type_index),
        })
    }
}

pub struct Part<'a, T> {
    pub part_name: &'static str,
    pub entries: &'a [T],
}

impl<'a, T> Part<'a, T> {
    pub fn entry(&self, entry_index: u32) -> Result<&'a T> {
        if let Some(entry) = self.entries.get(entry_index as usize) {
            Ok(entry)
        } else {
            Err(Error::bad_string(format!(
                "bad entry index in partition '{}'. index: {}, len: {}",
                self.part_name,
                entry_index,
                self.entries.len()
            )))
        }
    }
}

pub struct PartEntry {
    pub part_range: Range<usize>,
    pub count: usize,
    pub size: usize,
}
