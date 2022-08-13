#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

use core::mem::size_of;
use core::ops::Range;
use std::collections::HashMap;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

use ripper::*;

fn main() -> Result<()> {
    let f = std::fs::read("d:/ripper/out/input.cpp.ifc")?;

    let ifc = Ifc::load(f)?;


    let mut parts_sorted: Vec<_> = ifc.parts().iter().collect();
    parts_sorted.sort_unstable_by_key(|&(k, _)| k);

    println!("partitions:");
    for (part_name, part_entry) in parts_sorted.iter() {
        println!(
            "{:-40}     entry size: {:3}, num_entries: {}",
            part_name, part_entry.size, part_entry.count
        );
    }

    println!("loaded file");
    println!("global scope: (at index {})", ifc.global_scope());

    dump_fundamental_types(&ifc)?;

    dump_scope(&ifc, ifc.global_scope())?;

    Ok(())
}

fn dump_scope(ifc: &Ifc, scope: ScopeIndex) -> Result<()> {
    let scope_descriptor = ifc.get_scope_descriptor(scope)?;
    println!("scope descriptor = {:?}", scope_descriptor);

    let scope_members = ifc.scope_member();

    for member_index in 0..scope_descriptor.cardinality {
        let member_decl_index: DeclIndex = *scope_members.entry(member_index)?;

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
                let func_decl = ifc.decl_function().entry(member_decl_index.index())?;
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
    let part = ifc.type_fundamental();

    for ft in part.entries.iter() {
        println!("fundamental type: {:?}", ft);
    }

    Ok(())
}
