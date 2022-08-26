use super::*;

impl Gen {
    pub fn write_fn<'tcx>(&self, tcx: TyCtxt<'tcx>, local_def_id: LocalDefId) {
        let def_id = local_def_id.to_def_id();
        // let def_path = defs.def_path(local_def_id);
        let def_path = tcx.def_path_debug_str(local_def_id.to_def_id());
        let def_kind = tcx.def_kind(local_def_id);

        let sig = tcx.fn_sig(def_id);
        info!("sig: {:?}", sig);

        // println!("    at {:?}", tx.source_span_untracked(local_def_id));
    }
}
