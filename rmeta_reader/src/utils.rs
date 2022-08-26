use super::*;

pub fn peel_transparent_repr<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    let mut ty = ty;
    loop {
        match ty.kind() {
            TyKind::Adt(adt, _substs) => {
                let repr = adt.repr();
                if !repr.transparent() {
                    break;
                }

                if let Some(inner_field) = adt.all_fields().next() {
                    let field_ty = tcx.type_of(inner_field.did);
                    ty = field_ty;
                    continue;
                } else {
                    // It's #[repr(transparent)] but has no fields.
                    // Whatever, we ignore it.
                    break;
                }
            }

            _ => break,
        }
    }

    ty
}

pub fn peel_newtypes<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Ty<'tcx> {
    match ty.kind() {
        TyKind::Adt(adt, _substs) => {
            let mut has_real_field = false;
            for field in adt.all_fields() {
                let field_ty = tcx.type_of(field.did);

                // We require that all fields be sized.
                if !field_ty.is_sized(tcx.at(Default::default()), ParamEnv::empty()) {
                    return ty;
                }
            }

            let repr = adt.repr();
            if repr.transparent() {}

            ty
        }

        _ => ty,
    }
}
