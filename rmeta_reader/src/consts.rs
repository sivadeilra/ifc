use super::*;

impl Gen {
    pub fn write_const<'tcx>(&mut self, tcx: TyCtxt<'tcx>, local_def_id: LocalDefId) {
        let def_id = local_def_id.to_def_id();

        let vis = tcx.visibility(def_id);
        if !matches!(vis, Visibility::Public) {
            debug!("type is not visible");
            return;
        }

        writelnstr!(self.output.header, "// {}", tcx.def_path_str(def_id));

        let def_path = tcx.def_path(def_id);
        // println!("// def_path: {:?}", def_path);

        let simple_name = if let Some(n) = self.ro.get_simple_name(tcx, def_id) {
            n
        } else {
            return;
        };

        let original_def_ty = tcx.type_of(def_id);
        let def_ty = peel_transparent_repr(tcx, original_def_ty);

        match def_ty.kind() {
            TyKind::Str => {
                writelnstr!(self.output.header, "// it's a string");
            }

            TyKind::Char => {
                // Numeric constants are our favorites.
                match tcx.const_eval_poly(def_id) {
                    Ok(ConstValue::Scalar(Scalar::Int(scalar))) => {
                        writelnstr!(
                            self.output.header,
                            "constexpr char32_t {} = '{:?}';",
                            simple_name,
                            scalar
                        );
                    }
                    Ok(_) => {
                        writelnstr!(
                            self.output.header,
                            "// evaluated constant, but did not get the expected type"
                        );
                        return;
                    }
                    Err(e) => {
                        writelnstr!(self.output.header, "failed to evaluate constant: {:?}", e);
                        return;
                    }
                }
            }

            TyKind::Int(_) | TyKind::Uint(_) | TyKind::Float(_) => {
                // Numeric constants are our favorites.
                match tcx.const_eval_poly(def_id) {
                    Ok(ConstValue::Scalar(Scalar::Int(scalar))) => {
                        // let def_name = self.cxx_name_for(def_id);
                        let ty_str = self
                            .ro
                            .cxx_type_for(tcx, peel_transparent_repr(tcx, tcx.type_of(def_id)));
                        writelnstr!(
                            self.output.header,
                            "constexpr {} {} = {:?};",
                            ty_str,
                            simple_name,
                            scalar
                        );
                    }
                    Ok(_) => {
                        writelnstr!(
                            self.output.header,
                            "// evaluated constant, but did not get the expected type"
                        );
                        return;
                    }
                    Err(e) => {
                        writelnstr!(
                            self.output.header,
                            "// failed to evaluate constant: {:?}",
                            e
                        );
                        return;
                    }
                }
            }

            _ => {}
        }

        if false {
            match tcx.const_eval_poly(def_id) {
                Ok(ConstValue::Scalar(Scalar::Int(scalar))) => {
                    // let def_name = self.cxx_name_for(def_id);
                    let ty_str = self.ro.cxx_type_for(tcx, tcx.type_of(def_id));
                    writelnstr!(
                        self.output.header,
                        "constexpr {} {} = {:?};",
                        ty_str,
                        simple_name,
                        scalar
                    );
                }
                Ok(ConstValue::Scalar(Scalar::Ptr(p, n))) => {
                    writelnstr!(
                        self.output.header,
                        "// it's a constant pointer {:?} {:}",
                        p,
                        n
                    );
                }
                Ok(value) => {
                    writelnstr!(
                        self.output.header,
                        "// it's a constant (but not a scalar): {:?}",
                        value
                    );
                }
                Err(e) => {
                    writelnstr!(
                        self.output.header,
                        "// failed to evaluate constant: {:?}",
                        e
                    );
                }
            }
        }
    }
}
