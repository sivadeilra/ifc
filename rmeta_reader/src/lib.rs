//! Reads Rust metadata
//!
//! See https://doc.rust-lang.org/unstable-book/language-features/rustc-private.html
#![allow(unused_imports)]
#![allow(unused_variables)]
#![feature(rustc_private)]

extern crate rustc_ast;
extern crate rustc_codegen_ssa;
extern crate rustc_driver;
extern crate rustc_errors;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_metadata;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
extern crate rustc_target;

#[macro_use]
mod macros;

mod consts;
mod externs;
mod fns;
mod types;
mod utils;

#[cfg(test)]
mod tests;

use log::{debug, info};
use rustc_ast::ast;
use rustc_driver::Compilation;
use rustc_errors::registry::Registry;
use rustc_hash::FxHashMap;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::{DefId, LocalDefId};
use rustc_hir::definitions::DefPathData;
use rustc_interface::interface::Compiler;
use rustc_interface::Queries;
use rustc_metadata::creader::*;
use rustc_middle::mir::interpret::{ConstValue, Scalar};
use rustc_middle::ty::{FloatTy, IntTy, ParamEnv, Ty, TyCtxt, TyKind, UintTy, Visibility};
use rustc_session::config;
use rustc_session::DiagnosticOutput;
use rustc_session::Session;
use rustc_span::def_id::{CrateNum, LOCAL_CRATE};
use rustc_span::source_map::FileLoader;
use rustc_span::{Span, Symbol};
use rustc_target::abi::Abi;
use rustc_target::spec::{Target, TargetTriple};
use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Write;
use std::path::PathBuf;
use utils::*;

struct GenRo {
    cxx_ty_isize: String,
    cxx_ty_i8: String,
    cxx_ty_i16: String,
    cxx_ty_i32: String,
    cxx_ty_i64: String,
    cxx_ty_usize: String,
    cxx_ty_u8: String,
    cxx_ty_u16: String,
    cxx_ty_u32: String,
    cxx_ty_u64: String,
    cxx_ty_char32: String,
}

#[derive(Default)]
struct Gen {
    ro: GenRo,
    output: GenOutput,

    /// The set of crates that are mentioned in the public metadata of this crate,
    /// and which are visible from C++ definitions.
    ///
    /// This set can contain both direct and indirect dependencies of the current crate.
    required_crates: HashSet<CrateNum>,
}

impl Default for GenRo {
    fn default() -> Self {
        if false {
            Self {
                cxx_ty_isize: "signed __intptr".to_string(),
                cxx_ty_i8: "signed __int8".to_string(),
                cxx_ty_i16: "signed __int16".to_string(),
                cxx_ty_i32: "signed __int32".to_string(),
                cxx_ty_i64: "signed __int64".to_string(),
                cxx_ty_usize: "unsigned __intptr".to_string(),
                cxx_ty_u8: "unsigned __int8".to_string(),
                cxx_ty_u16: "unsigned __int16".to_string(),
                cxx_ty_u32: "unsigned __int32".to_string(),
                cxx_ty_u64: "unsigned __int64".to_string(),
                cxx_ty_char32: "unsigned __int32".to_string(),
            }
        } else {
            Self {
                cxx_ty_isize: "ssize_t".to_string(),
                cxx_ty_i8: "signed char".to_string(),
                cxx_ty_i16: "short".to_string(),
                cxx_ty_i32: "int".to_string(),
                cxx_ty_i64: "long long".to_string(),
                cxx_ty_usize: "size_t".to_string(),
                cxx_ty_u8: "unsigned char".to_string(),
                cxx_ty_u16: "unsigned short".to_string(),
                cxx_ty_u32: "unsigned int".to_string(),
                cxx_ty_u64: "unsigned long long".to_string(),
                cxx_ty_char32: "char32_t".to_string(),
            }
        }
    }
}

impl GenRo {
    fn get_simple_name<'tcx>(&self, tcx: TyCtxt<'tcx>, def_id: DefId) -> Option<String> {
        let def_path = tcx.def_path(def_id);
        // info!("data: {:?}", def_path.data);
        if def_path.data.len() != 1 {
            info!(
                "// path has wrong number of elements (is {}, wanted 1)",
                def_path.data.len()
            );
            return None;
        }
        let d0 = &def_path.data[0];
        // info!("// d0 = {:?}", d0);
        match &d0.data {
            DefPathData::ValueNs(name) => Some(name.to_string()),
            _ => None,
        }
    }
    fn cxx_type_for<'s>(&'s self, tcx: TyCtxt, ty: Ty) -> Cow<'s, str> {
        match ty.kind() {
            TyKind::Bool => Cow::Borrowed("bool"),
            TyKind::Str => Cow::Borrowed("const char32_t*"),
            TyKind::Char => Cow::Borrowed(self.cxx_ty_char32.as_str()),
            TyKind::Float(FloatTy::F32) => Cow::Borrowed("float"),
            TyKind::Float(FloatTy::F64) => Cow::Borrowed("double"),
            TyKind::Int(IntTy::Isize) => Cow::Borrowed(self.cxx_ty_isize.as_str()),
            TyKind::Int(IntTy::I8) => Cow::Borrowed(self.cxx_ty_i8.as_str()),
            TyKind::Int(IntTy::I16) => Cow::Borrowed(self.cxx_ty_i16.as_str()),
            TyKind::Int(IntTy::I32) => Cow::Borrowed(self.cxx_ty_i32.as_str()),
            TyKind::Int(IntTy::I64) => Cow::Borrowed(self.cxx_ty_i64.as_str()),
            TyKind::Uint(UintTy::Usize) => Cow::Borrowed(self.cxx_ty_usize.as_str()),
            TyKind::Uint(UintTy::U8) => Cow::Borrowed(self.cxx_ty_u8.as_str()),
            TyKind::Uint(UintTy::U16) => Cow::Borrowed(self.cxx_ty_u16.as_str()),
            TyKind::Uint(UintTy::U32) => Cow::Borrowed(self.cxx_ty_u32.as_str()),
            TyKind::Uint(UintTy::U64) => Cow::Borrowed(self.cxx_ty_u64.as_str()),
            TyKind::Adt(adt, _) => Cow::Owned(format!("{:?}", adt)),
            _ => Cow::Owned(format!("?ty {:?}", ty)),
        }
    }
}

impl Gen {
    fn requires_def_id(&mut self, def_id: DefId) {
        self.requires_crate(def_id.krate);
    }

    fn requires_crate(&mut self, krate: CrateNum) {
        if krate == LOCAL_CRATE {
            return;
        }

        self.required_crates.insert(krate);
    }
}

#[derive(Default)]
struct GenOutput {
    header: String,
}

impl rustc_driver::Callbacks for Gen {
    fn after_analysis<'tcx>(&mut self, c: &Compiler, queries: &'tcx Queries<'tcx>) -> Compilation {
        info!("after_analysis");

        let mut gc = queries.global_ctxt().unwrap().peek_mut();
        gc.enter(|tcx| {
            info!("we got a TyCtxt");

            let cstore = tcx.cstore_untracked();

            let defs = tcx.definitions_untracked();
            // info!("def_index_count = {}", defs.def_index_count());

            for local_def_id in defs.iter_local_def_id() {
                let def_id = local_def_id.to_def_id();
                // let def_path = defs.def_path(local_def_id);
                let def_path = tcx.def_path_debug_str(local_def_id.to_def_id());
                info!(
                    "local def: {:?} - {:?}, {:?}",
                    tcx.def_kind(local_def_id),
                    local_def_id,
                    def_path
                );

                let def_kind = tcx.def_kind(local_def_id);
                match def_kind {
                    DefKind::Const => self.write_const(tcx, local_def_id),
                    DefKind::ExternCrate => self.process_extern_crate(tcx, local_def_id),
                    DefKind::Struct => self.write_struct(tcx, local_def_id),
                    DefKind::Union => self.write_union(tcx, local_def_id),
                    DefKind::Enum => self.write_enum(tcx, local_def_id),
                    DefKind::Fn => self.write_fn(tcx, local_def_id),

                    _ => {}
                }

                // info!("    at {:?}", tx.source_span_untracked(local_def_id));
            }
        });

        Compilation::Stop
    }
}
