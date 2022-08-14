#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]

use core::mem::size_of;
use core::ops::Range;
use std::collections::HashMap;
use structopt::StructOpt;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};
use anyhow::Result;

use ifc::*;

#[derive(StructOpt)]
struct Options {
    /// Filename to read. This is usually `<something>.ifc`.
    ifc: String,

    /// Show everything possible.
    #[structopt(short = "a", long = "all")]
    all: bool,
}

fn main() -> Result<()> {
    let options = Options::from_args();

    let f = std::fs::read(&options.ifc)?;

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
    println!();

    println!("global scope: (at index {})", ifc.global_scope());

    dump_summary(&ifc)?;
    dump_command_line(&ifc)?;
    dump_name_source_file(&ifc)?;
    dump_fundamental_types(&ifc)?;

    // println!("Global scope: ({})", ifc.global_scope());
    // dump_scope(&ifc, ifc.global_scope(), 20)?;
    // println!();

    println!("Scopes:");
    dump_scopes(&ifc)?;
    println!();

    dump_attr_basic(&ifc)?;

    Ok(())
}

fn dump_summary(ifc: &Ifc) -> Result<()> {
    let unit = ifc.file_header().unit;
    println!("Unit = {} 0x{:x}", unit, unit);

    Ok(())
}

fn dump_scopes(ifc: &Ifc) -> Result<()> {
    for i in 0..ifc.scope_desc().entries.len() {
        dump_scope(ifc, i as ScopeIndex + 1, 10)?;
    }
    Ok(())
}

fn dump_scope(ifc: &Ifc, parent_scope: ScopeIndex, max_depth: u32) -> Result<()> {
    if parent_scope == 0 {
        println!("Invalid scope (zero)");
        return Ok(());
    }

    println!(
        "Scope #{}{}",
        parent_scope,
        if parent_scope == ifc.file_header().global_scope {
            " - Global scope"
        } else {
            ""
        }
    );

    // `scope.descriptor` gives us the start and length of the region in `scope.members` where
    // the members for this scope can be found.
    let scope_descriptor = ifc.scope_desc().entry(parent_scope - 1)?;

    if max_depth == 0 {
        println!("Max depth exceeded!");
        return Ok(());
    }
    let max_depth = max_depth - 1;

    println!("scope descriptor = {:?}", scope_descriptor);

    let scope_members = ifc.scope_member();

    for member_index in
        scope_descriptor.start..scope_descriptor.start + scope_descriptor.cardinality
    {
        let member_decl_index: DeclIndex = *scope_members.entry(member_index)?;

        println!(
            "member {}: decl_index = {:?}",
            member_index, member_decl_index
        );

        match member_decl_index.tag() {
            DeclSort::ALIAS => {
                let decl_alias = ifc.decl_alias().entry(member_decl_index.index())?;
                let alias_name = ifc.get_string(decl_alias.name)?;
                println!("    alias: {}", alias_name);
                println!("{:#?}", decl_alias);
            }

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

            DeclSort::SCOPE => {
                let nested_scope = ifc.decl_scope().entry(member_decl_index.index())?;
                let nested_scope_name = ifc.get_string(nested_scope.name.index())?;

                // What kind of scope is it?
                if ifc.is_type_namespace(nested_scope.ty)? {
                    println!("    namespace {} {{ ... }}", nested_scope_name);
                } else {
                    println!(
                        "    nested scope: name: {:#?} - {:?}",
                        nested_scope_name, nested_scope.ty
                    );
                }

                // println!("{:#?}", nested_scope);

                // if scope.initializer != 0 {
                //     dump_scope(ifc, scope.initializer, max_depth)?;
                // }
            }

            DeclSort::FIELD => {
                let field = ifc.decl_field().entry(member_decl_index.index())?;
                let field_name = ifc.get_string(field.name)?;
                let field_type_string = ifc.get_type_string(field.ty)?;
                println!("    field: {} : {}", field_name, field_type_string);
            }

            DeclSort::METHOD => {}

            DeclSort::ENUMERATION => {
                let en = ifc.decl_enum().entry(member_decl_index.index())?;
                let en_name = ifc.get_string(en.name)?;
                println!("    enum: {}", en_name);

                for var_index in en.initializer.to_range() {
                    let var = ifc.decl_enumerator().entry(var_index)?;
                    let var_name = ifc.get_string(var.name)?;
                    println!("      {}", var_name);
                }
            }

            _ => {
                nyi!();
                println!("unknown decl: {:?}", member_decl_index);
            }
        }
        println!();

        if member_index == 1000 {
            break;
        }
    }

    Ok(())
}

// IFC files contain `command_line` partition, but this is undocumented.
fn dump_command_line(ifc: &Ifc) -> Result<()> {
    let cmd = ifc.command_line();
    if !cmd.entries.is_empty() {
        println!("Command line (from `command_line`):");
        for arg in ifc.command_line().entries.iter() {
            let s = ifc.get_string(*arg)?;
            println!("    {}", s);
        }
    }
    println!();
    Ok(())
}

fn dump_name_source_file(ifc: &Ifc) -> Result<()> {
    println!("Source files:");

    for (i, entry) in ifc.name_source_file().entries.iter().enumerate() {
        if i == 0 && entry.path == 0 {
            continue;
        }

        if entry.path == 0 {
            println!(
                "unexpected: entry #{} in name.source-file contains empty path",
                i
            );
            continue;
        }

        let path = ifc.get_string(entry.path)?;
        println!("    {}", path);
        if entry.guard != 0 {
            let guard = ifc.get_string(entry.guard)?;
            println!("    guard {}", guard);
        }
    }

    println!();
    Ok(())
}

fn dump_fundamental_types(ifc: &Ifc) -> Result<()> {
    println!("Fundamental types (`type.fundamental`):");
    let part = ifc.type_fundamental();
    for ft in part.entries.iter() {
        println!("    {:?}", ft);
    }

    println!();
    Ok(())
}

fn dump_attr_basic(ifc: &Ifc) -> Result<()> {
    if ifc.attr_basic().entries.is_empty() {
        return Ok(());
    }

    println!("Attributes (basic) (from `attr.basic`):");
    for attr in ifc.attr_basic().entries.iter() {
        println!("    attr: {:#?}", attr);
        match attr.sort {
            WordSort::UNKNOWN => println!("    unknown"),
            WordSort::DIRECTIVE => println!("    directive"),
            WordSort::IDENTIFIER => {}
            _ => {
                println!("    ???");
            }
        }
    }
    println!();

    Ok(())
}
