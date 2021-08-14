#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

use core::mem::size_of;
use core::ops::Range;
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

#[macro_use]
pub mod macros;

pub mod error;
pub mod ifc;

use error::*;
use ifc::*;

fn main() -> Result<()> {
    // let f = std::fs::read("d:/ifc/out/vector.ifc")?;
    let f = std::fs::read("d:/ripper/out/input.cpp.ifc")?;

    let fs = f.as_slice();

    let sig = read_struct_at::<[u8; 4]>(&fs[0..])?;
    if sig != IFC_FILE_SIGNATURE {
        println!("File does not have IFC file signature.");
        return Ok(());
    }

    let file_header = read_struct_at::<FileHeader>(&fs[4..])?;
    println!("File header: {:#?}", file_header);

    let strings_range = file_header.string_table_bytes as usize
        ..file_header.string_table_bytes as usize + file_header.string_table_size as usize;
    if f.get(strings_range.clone()).is_none() {
        return Err(Error::bad("string table range is not valid"));
    }

    let mut ifc = Ifc {
        data: f,
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
                + partition_summary.cardinality as usize * partition_summary.entity_size as usize;
        if ifc.data.get(part_range.clone()).is_none() {
            return Err(Error::bad_string(format!(
                "partition {} {:?} is invalid; its range is outside the ifc file size",
                i, partition_name
            )));
        }

        ifc.parts.set_part_info(partition_name, &partition_summary);

        ifc.parts_map.insert(
            partition_name.to_string(),
            PartEntry {
                part_range,
                count: partition_summary.cardinality as usize,
                size: partition_summary.entity_size as usize,
            },
        );
    }

    let mut parts_sorted: Vec<_> = ifc.parts_map.iter().collect();
    parts_sorted.sort_unstable_by_key(|&(k, _)| k);

    println!("partitions:");
    for (part_name, part_entry) in parts_sorted.iter() {
        println!(
            "{:-40}     entry size: {:3}, num_entries: {}",
            part_name, part_entry.size, part_entry.count
        );
    }

    println!("loaded file");
    println!("global scope: (at index {})", ifc.file_header.global_scope);

    dump_scope(&ifc, ifc.file_header.global_scope)?;

    Ok(())
}

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

fn dump_scope(ifc: &Ifc, scope: ScopeIndex) -> Result<()> {
    let scope_descriptor = ifc.get_scope_descriptor(scope)?;
    println!("scope descriptor = {:?}", scope_descriptor);

    let scope_members = ifc.scope_member()?;

    for member_index in 0..scope_descriptor.cardinality {
        let member_desc_bytes = scope_members.entry(member_index)?;
        let member_decl_index: DeclIndex = read_struct_at(member_desc_bytes)?;

        println!(
            "member {}: decl_index = {:?}",
            member_index, member_decl_index
        );

        match member_decl_index.tag() {
            /*
            DeclSort::ALIAS => {
                // 8.2.9
                let decl_alias: DeclAlias =
                    read_struct_at(ifc.decl_alias()?.entry(member_decl_index.index())?)?;
                println!("{:#?}", decl_alias);
            }
            */
            DeclSort::FUNCTION => {
                // 8.2.16
                let func_decl: DeclFunc =
                    read_struct_at(ifc.decl_function()?.entry(member_decl_index.index())?)?;
                let func_name = match func_decl.name.tag() {
                    NameSort::IDENTIFIER => ifc.get_string(func_decl.name.index())?.to_string(),
                    _ => format!("{:?}", func_decl.name),
                };
                // println!("function \"{}\" = {:#?}", func_name, func_decl);

                let type_str = ifc.get_type_string(func_decl.type_)?;
                println!("function: \"{}\" = type = {}", func_name, type_str);
            }
            _ => {}
        }

        if member_index == 1000 {
            break;
        }
    }

    Ok(())
}

fn dump_fundamental_types(ifc: &Ifc) -> Result<()> {
    let part = ifc.type_fundamental()?;



    Ok(())
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

struct Ifc {
    data: Vec<u8>,
    file_header: FileHeader,
    strings_range: Range<usize>,

    parts_map: HashMap<String, PartEntry>,
    parts: Parts,
}

impl Ifc {
    pub fn get_string(&self, text_offset: TextOffset) -> Result<&str> {
        StringTable {
            strings: &self.data[self.strings_range.clone()],
        }
        .get_string(text_offset)
    }

    pub fn get_part_by_name_opt<'a, 'p>(&'a self, name: &'p str) -> Option<Part<'a, 'p>> {
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

    pub fn get_scope_descriptor(&self, scope_index: ScopeIndex) -> Result<ScopeDescriptor> {
        let scope_descriptor_bytes = self.get_part_entry(part_names::scope_desc, scope_index)?;
        let scope_descriptor = read_struct_at::<ScopeDescriptor>(scope_descriptor_bytes)?;
        Ok(scope_descriptor)
    }

    pub fn type_heap_lookup(&self, index: Index) -> Result<TypeIndex> {
        Ok(read_struct_at::<TypeIndex>(
            self.heap_type()?.entry(index)?,
        )?)
    }

    pub fn get_type_string(&self, type_index: TypeIndex) -> Result<String> {
        use core::fmt::Write;
        Ok(match type_index.tag() {
            TypeSort::FUNCTION => {
                let type_function_part = self.type_function()?;
                let type_func: FunctionType =
                    read_struct_at(type_function_part.entry(type_index.index())?)?;

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
                let type_fundamental: FundamentalType =
                    read_struct_at(self.type_fundamental()?.entry(type_index.index())?)?;
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
                let type_tuple: TupleType =
                    read_struct_at(self.type_tuple()?.entry(type_index.index())?)?;
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
                let qualified_type: QualifiedType =
                    read_struct_at(self.type_qualified()?.entry(type_index.index())?)?;
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
                let pointee_type: TypeIndex =
                    read_struct_at(self.type_pointer()?.entry(type_index.index())?)?;
                let mut pointee_type_str = self.get_type_string(pointee_type)?;
                pointee_type_str.push_str("*");
                pointee_type_str
            }

            _ => format!("{:?}", type_index),
        })
    }
}

pub struct Part<'a, 'p> {
    part_name: &'p str,
    part_data: &'a [u8],
    count: usize,
    size: usize,
}

impl<'a, 'p> Part<'a, 'p> {
    pub fn entry(&self, entry_index: u32) -> Result<&'a [u8]> {
        let entry_start = self.size as usize * entry_index as usize;
        if let Some(entry_bytes) = self
            .part_data
            .get(entry_start..entry_start + self.size as usize)
        {
            Ok(entry_bytes)
        } else {
            Err(Error::bad_string(format!(
                "bad entry index in partition '{}': {}.  part_data.len = {}, count = {}, size = {}",
                self.part_name,
                entry_index,
                self.part_data.len(),
                self.count,
                self.size
            )))
        }
    }
}

struct PartEntry {
    part_range: Range<usize>,
    count: usize,
    size: usize,
}
