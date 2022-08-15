use super::*;
use core::mem::size_of;
use anyhow::Result;
use log::debug;

// Partition

#[repr(C)]
#[derive(FromBytes, AsBytes, Clone, Default, Debug)]
pub struct PartitionSummary {
    // An index into the string pool. The name of the partition.
    pub name: TextOffset,
    // File offset in bytes of the partition relative to the beginning of the IFC file.
    pub offset: ByteOffset,
    // The number of items in the partition.
    pub cardinality: Cardinality,
    // The (common) size of an item in the partition.
    pub entity_size: EntitySize,
}

macro_rules! part_info {
    ($(
        $part_ident:ident,          // identifier in rust source code, e.g. decl_func
        $part_name:expr,            // partition name as in ifc, e.g. "decl.func",
        $part_record:ty             // type of the record
        ;
    )*) => {

        impl Ifc {
            $(
                pub fn $part_ident<'a>(&'a self) -> Part<'a, $part_record> {
                    Part::<$part_record> {
                        part_name: $part_name,
                        entries: &self.parts.$part_ident,
                    }
                }
            )*
        }

        #[derive(Default)]
        pub struct Parts {
            $(
                pub $part_ident: Vec<$part_record>,
            )*
        }

        impl Parts {
            pub fn load_part_data(&mut self, name: &str, part_data: &[u8], num_records: usize, record_size: usize) -> Result<()> {
                match name {
                    $(
                        $part_name => {
                            self.$part_ident = convert_record_data::<$part_record>($part_name, part_data, num_records, record_size);
                            Ok(())
                        }
                    )*
                    _ => {
                        // We don't recognize this partition by name. That's ok.
                        Ok(())
                    }
                }
            }
        }
    }
}

fn convert_record_data<T>(
    part_name: &str,
    part_data: &[u8],
    num_records: usize,
    record_size: usize,
) -> Vec<T>
where
    T: FromBytes + AsBytes,
{
    let expected_record_size = size_of::<T>();
    assert_eq!(part_data.len(), num_records * record_size);

    // There are three cases to consider:
    // * The records in the file are smaller than we expected.
    //   In this case, we zero-extend each record.
    // * The records in the file are larger than we expected.
    //   In this case, we truncate each record.
    // * The records are exactly the size we expected.

    let mut vec: Vec<T> = Vec::with_capacity(num_records);

    // TODO: Use zerocopy support to efficiently zero-extend this.
    vec.resize_with(num_records, new_zeroed);

    if expected_record_size == record_size {
        debug!(
            "loading partition {}, {} records, exact size",
            part_name, num_records
        );

        vec.as_bytes_mut()
            .copy_from_slice(&part_data[..num_records * record_size]);
    } else if expected_record_size < record_size {
        // Truncate each record.
        println!(
            "loading partition {}, {} records, truncating records from {} bytes to {}",
            part_name, num_records, record_size, expected_record_size
        );
        for (dst, src) in vec
            .as_bytes_mut()
            .chunks_exact_mut(expected_record_size)
            .zip(part_data.chunks_exact(record_size))
        {
            dst.copy_from_slice(&src[..expected_record_size]);
        }
    } else {
        // Zero-fill (implicitly) each record. Copy only what is valid.
        println!(
            "loading partition {}, {} records, expanding records from {} bytes to {}",
            part_name, num_records, record_size, expected_record_size
        );
        for (dst, src) in vec
            .as_bytes_mut()
            .chunks_exact_mut(expected_record_size)
            .zip(part_data.chunks_exact(record_size))
        {
            dst[..expected_record_size].copy_from_slice(src);
        }
    }

    vec
}

part_info! {
    decl_alias, "decl.alias", DeclAlias;
    decl_function, "decl.function", DeclFunc;
    decl_scope, "decl.scope", DeclScope;
    decl_field, "decl.field", DeclField;
    decl_enum, "decl.enum", DeclEnum;
    decl_enumerator, "decl.enumerator", DeclEnumerator;
    decl_var, "decl.variable", DeclVar;

    heap_type, "heap.type", TypeIndex;
    scope_desc, "scope.desc", ScopeDescriptor;
    scope_member, "scope.member", DeclIndex;
    type_function, "type.function", FunctionType;
    type_fundamental, "type.fundamental", FundamentalType;
    type_pointer, "type.pointer", TypeIndex;
    type_qualified, "type.qualified", QualifiedType;
    type_tuple, "type.tuple", TupleType;
    type_array, "type.array", TypeArray;
    name_source_file, "name.source-file", NameSourceFile;
    command_line, "command_line", TextOffset;

    expr_literal, "expr.literal", ExprLiteral;
    expr_dyad, "expr.dyad", ExprDyad;

    const_i64, "const.i64", u64;
    const_f64, "const.f64", ConstF64;

    // Attributes using AttrSort::Basic
    attr_basic, "attr.basic", Word;
}
