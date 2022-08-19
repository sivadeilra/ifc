#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![forbid(unused_must_use)]
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use core::mem::size_of;
use core::ops::Range;
use ifc::*;
use log::trace;
use options::Options;
use regex::Regex;
use std::collections::HashMap;
use structopt::StructOpt;
use zerocopy::{AsBytes, FromBytes, LayoutVerified};

mod options;
mod parts;
mod pp;
mod summary;

fn main() -> Result<()> {
    let mut options = options::Options::from_args();

    // If the user didn't specify anything, then show the summary by default.
    if !options.all
        && !options.sources
        && !options.defines
        && !options.enums
        && !options.functions
        && !options.typedefs
        && !options.structs
        && !options.parts
        && !options.funtypes
    {
        options.summary = true;
        options.parts = true;
    }

    if options.all {
        options.sources = true;
        options.defines = true;
        options.functions = true;
        options.funtypes = true;
        options.enums = true;
        options.typedefs = true;
        options.structs = true;
        options.parts = true;
    }

    let rx_opt: Option<Regex> = if let Some(w) = options.where_.as_deref() {
        Some(
            regex::RegexBuilder::new(w)
                .case_insensitive(!options.wcase)
                .build()
                .with_context(|| "The filter regex is invalid.".to_string())?,
        )
    } else {
        None
    };
    let mut num_matches: u64 = 0;

    let mut matcher = |name: &str| -> bool {
        if let Some(rx) = rx_opt.as_ref() {
            if rx.is_match(name) {
                num_matches += 1;
                true
            } else {
                false
            }
        } else {
            true
        }
    };

    let f = std::fs::read(&options.ifc)?;

    let ifc = Ifc::load(f)?;

    if options.parts {
        parts::dump_parts(&ifc)?;
    }

    if options.summary {
        summary::dump_summary(&ifc)?;
        dump_command_line(&ifc)?;
    }

    if options.sources {
        dump_name_source_file(&ifc)?;
    }

    if options.funtypes {
        dump_fundamental_types(&ifc)?;
    }

    if options.defines {
        pp::dump_pp(&ifc, &mut matcher)?;
    }

    let needs_scope = options.functions || options.enums || options.structs || options.typedefs;
    if needs_scope {
        dump_scope(&ifc, ifc.global_scope(), &options, 20)?;
    }

    // dump_attr_basic(&ifc)?;

    if rx_opt.is_some() {
        println!();
        println!("Number of matches: {}", num_matches);
    }

    Ok(())
}

#[cfg(nope)]
fn dump_scopes(ifc: &Ifc) -> Result<()> {
    for i in 0..ifc.scope_desc().entries.len() {
        dump_scope(ifc, i as ScopeIndex + 1, 10)?;
    }
    Ok(())
}

fn dump_scope(
    ifc: &Ifc,
    parent_scope: ScopeIndex,
    options: &Options,
    max_depth: u32,
) -> Result<()> {
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

    trace!("scope descriptor = {:?}", scope_descriptor);

    let scope_members = ifc.scope_member();

    for member_index in
        scope_descriptor.start..scope_descriptor.start + scope_descriptor.cardinality
    {
        let member_decl_index: DeclIndex = *scope_members.entry(member_index)?;

        trace!(
            "member {}: decl_index = {:?}",
            member_index,
            member_decl_index
        );

        match member_decl_index.tag() {
            DeclSort::ALIAS => {
                if options.typedefs {
                    let decl_alias = ifc.decl_alias().entry(member_decl_index.index())?;
                    let alias_name = ifc.get_string(decl_alias.name)?;
                    println!("    alias: {}", alias_name);
                    println!("{:#?}", decl_alias);
                }
            }

            DeclSort::FUNCTION | DeclSort::METHOD => {
                if options.functions {
                    let func_decl = if member_decl_index.tag() == DeclSort::FUNCTION {
                        ifc.decl_function().entry(member_decl_index.index())?
                    } else {
                        ifc.decl_method().entry(member_decl_index.index())?
                    };
                    let func_name = ifc.get_name_string(func_decl.name)?;
                    let type_str = ifc.get_type_string(func_decl.type_)?;
                    println!("function: {} : {}", func_name, type_str);
                }
            }

            DeclSort::SCOPE => {
                let nested_scope = ifc.decl_scope().entry(member_decl_index.index())?;
                let nested_scope_name = ifc.get_string(nested_scope.name.index())?;

                // What kind of scope is it?
                if ifc.is_type_namespace(nested_scope.ty)? {
                    // It's a namespace. We always recurse into namespaces.
                    dump_scope(ifc, nested_scope.initializer, options, max_depth - 1)?;
                } else {
                    if options.structs {
                        // It's a nested struct/class.
                        println!("struct {} {{", nested_scope_name);
                        dump_scope(ifc, nested_scope.initializer, options, max_depth - 1)?;
                        println!("}} // struct {}", nested_scope_name);
                        println!();
                    }
                }
            }

            DeclSort::FIELD => {
                // If we got here, then we are inside a struct/class scope, and we always want
                // to show the fields.
                let field = ifc.decl_field().entry(member_decl_index.index())?;
                let field_name = ifc.get_string(field.name)?;
                let field_type_string = ifc.get_type_string(field.ty)?;
                println!("    field: {} : {}", field_name, field_type_string);
            }

            DeclSort::ENUMERATION => {
                if options.enums {
                    let en = ifc.decl_enum().entry(member_decl_index.index())?;
                    let en_name = ifc.get_string(en.name)?;
                    println!("    enum: {}", en_name);

                    for var_index in en.initializer.to_range() {
                        let var = ifc.decl_enumerator().entry(var_index)?;
                        let var_name = ifc.get_string(var.name)?;
                        println!("      {}", var_name);
                    }
                }
            }

            DeclSort::VARIABLE => {
                // TODO
            }

            DeclSort::TEMPLATE => {
                // TODO
            }

            DeclSort::EXPLICIT_SPECIALIZATION => {
                // TODO
            }

            DeclSort::INTRINSIC => {
                // ignore for now
            }

            _ => {
                nyi!();
                println!("unknown decl: {:?}", member_decl_index);
            }
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
