use super::*;

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
    ($($part_ident:ident, $part_name:expr;)*) => {

        #[allow(non_upper_case_globals)]
        pub mod part_names {
            $(
                pub const $part_ident: &str = $part_name;
            )*
        }

        impl Ifc {
            $(
                pub fn $part_ident<'a>(&'a self) -> Result<Part<'a, 'static>> {
                    self.get_part_by_name($part_name)
                }
            )*
        }

        #[derive(Clone, Default)]
        pub struct Parts {
            $(
                pub $part_ident: PartStuff,
            )*
        }

        impl Parts {
            pub fn get_part_mut(&mut self, name: &str) -> Option<&mut PartStuff> {
                match name {
                    $( $part_name => Some(&mut self.$part_ident), )*
                    // We don't recognize this partition by name. That's ok.
                    _ => None,
                }
            }
        }
    }
}

impl Parts {
    pub fn set_part_info(&mut self, name: &str, summary: &PartitionSummary) {
        if let Some(part) = self.get_part_mut(name) {
            *part = PartStuff {
                range: summary.offset as usize
                    ..summary.offset as usize
                        + summary.cardinality as usize * summary.entity_size as usize,
                entity_size: summary.entity_size as usize,
                count: summary.cardinality as usize,
            };
        }
    }
}

#[derive(Default, Clone)]
pub struct PartStuff {
    // byte range, relative to start of IFC
    pub range: Range<usize>,
    pub entity_size: usize,
    pub count: usize,
}

part_info! {
    scope_desc, "scope.desc";
    scope_member, "scope.member";
    decl_alias, "decl.alias";
    decl_function, "decl.function";
    type_function, "type.function";
    type_fundamental, "type.fundamental";
    type_qualified, "type.qualified";
    type_pointer, "type.pointer";
    type_tuple, "type.tuple";
    heap_type, "heap.type";
}
