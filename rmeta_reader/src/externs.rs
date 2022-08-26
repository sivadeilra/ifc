//! Code that handles `extern crate foo;` references to other crates.
//!
//! We examine each crate. Some crates are ordinary Rust crates. Some are Rust crates that
//! were generated from C++ header files (actually, from IFC files).
//!
//! For extern crates that were generated from C++ header files, we emit an `#include "foo.h"`
//! statement that points to the original header file (or files?).
//!
//! For extern crates that are ordinary Rust crates, we need to determine whether there is
//! _already_ a header file that describes that Rust crate.  If there is, then we emit an
//! `#include` directive for it.  If there isn't, then the compilation fails, and we explain
//! to the user that they need to provide a header file for it.

use super::*;

impl Gen {
    pub fn process_extern_crate<'tcx>(&self, tcx: TyCtxt<'tcx>, local_def_id: LocalDefId) {
        let def_id = local_def_id.to_def_id();
        // let def_path = defs.def_path(local_def_id);
        let def_path = tcx.def_path_debug_str(local_def_id.to_def_id());
        println!(
            "local def: {:?} - {:?}, {:?}",
            tcx.def_kind(local_def_id),
            local_def_id,
            def_path
        );

        let def_kind = tcx.def_kind(local_def_id);
        match def_kind {
            DefKind::ExternCrate => {
                println!("// extern crate {}", def_path);
                println!("#include \"rust.{}.h\"", def_path);
            }

            _ => {}
        }
    }
}
