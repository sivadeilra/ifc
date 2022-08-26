use super::*;

impl Gen {
    pub fn write_struct<'tcx>(&mut self, tcx: TyCtxt<'tcx>, local_def_id: LocalDefId) {
        let def_id = local_def_id.to_def_id();

        let vis = tcx.visibility(def_id);
        if !matches!(vis, Visibility::Public) {
            debug!("def is not visible");
            return;
        }

        let adt_def = tcx.adt_def(def_id);
        let repr = adt_def.repr();

        // The struct must be annotated with either #[repr(C)] or #[repr(transparent)]
        if repr.c() {
            debug!("type has #[repr(C)]");
        } else if repr.transparent() {
            debug!("type has #[repr(transparent)]");
        } else {
            debug!("type does not have #[repr(C)] or #[repr(transparent] - ignoring type");
            return;
        }

        let struct_ty = tcx.type_of(def_id);

        let struct_name = self.ro.cxx_type_for(tcx, struct_ty);

        writelnstr!(self.output.header, "struct {} {{", struct_name);
        for field in adt_def.all_fields() {
            let field_ty = tcx.type_of(field.did);
            let field_ty = peel_transparent_repr(tcx, field_ty);
            writelnstr!(self.output.header, "    {} {};", self.ro.cxx_type_for(tcx, field_ty), field.name);
        }

        writestr!(self.output.header, "}};\n\n");
    }

    pub fn write_enum<'tcx>(&self, tcx: TyCtxt<'tcx>, local_def_id: LocalDefId) {
        let def_id = local_def_id.to_def_id();
        // let def_path = defs.def_path(local_def_id);
        let def_path = tcx.def_path_debug_str(local_def_id.to_def_id());
        let def_kind = tcx.def_kind(local_def_id);
        match def_kind {
            DefKind::Struct => {
                let adt_def = tcx.adt_def(def_id);
                let repr = adt_def.repr();
                println!("// struct, repr {:?}", repr);
                for field in adt_def.all_fields() {
                    let field_ty = tcx.type_of(field.did);
                    let field_ty = peel_transparent_repr(tcx, field_ty);
                    println!("    {} {};", self.ro.cxx_type_for(tcx, field_ty), field.name);
                }
            }

            DefKind::Union => {
                println!("// union");
            }

            DefKind::Enum => {
                println!("// enum");
            }

            _ => {}
        }

        // println!("    at {:?}", tx.source_span_untracked(local_def_id));
    }

    pub fn write_union<'tcx>(&self, tcx: TyCtxt<'tcx>, local_def_id: LocalDefId) {}
}
