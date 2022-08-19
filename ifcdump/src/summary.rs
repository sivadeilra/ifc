use super::*;

pub fn dump_summary(ifc: &Ifc) -> Result<()> {
    let unit = ifc.file_header().unit;
    println!("Unit = {} 0x{:x}", unit, unit);
    let mut totals = Totals::default();
    count_totals_in_scope(ifc, &mut totals, ifc.file_header().global_scope)?;

    totals.object_macros = ifc.macro_object_like().entries.len() as u64;
    totals.function_macros = ifc.macro_function_like().entries.len() as u64;

    show_totals(&totals);
    Ok(())
}

fn show_totals(totals: &Totals) {
    println!("{:#?}", totals);
}

fn count_totals_in_scope(ifc: &Ifc, totals: &mut Totals, scope: ScopeIndex) -> Result<()> {
    if scope == 0 {
        println!("Invalid scope (zero)");
        return Ok(());
    }

    let scope_descriptor = ifc.scope_desc().entry(scope - 1)?;
    let scope_members = ifc.scope_member();

    for member_index in
        scope_descriptor.start..scope_descriptor.start + scope_descriptor.cardinality
    {
        let member_decl_index = scope_members.entry(member_index)?;

        match member_decl_index.tag() {
            DeclSort::ALIAS => totals.typedefs += 1,
            DeclSort::FUNCTION => totals.functions += 1,
            DeclSort::METHOD => totals.methods += 1,
            DeclSort::ENUMERATION => totals.enums += 1,
            DeclSort::FIELD => totals.fields += 1,
            DeclSort::VARIABLE => totals.variables += 1,
            DeclSort::TEMPLATE => totals.templates += 1,
            DeclSort::INTRINSIC => totals.intrinsics += 1,
            DeclSort::BITFIELD => totals.bitfields += 1,
            DeclSort::EXPLICIT_SPECIALIZATION => totals.explicit_specialization += 1,

            DeclSort::SCOPE => {
                let nested_scope = ifc.decl_scope().entry(member_decl_index.index())?;
                let nested_scope_name = ifc.get_string(nested_scope.name.index())?;

                // What kind of scope is it?
                if ifc.is_type_namespace(nested_scope.ty)? {
                    totals.namespaces += 1;
                    if nested_scope.initializer != 0 {
                        count_totals_in_scope(ifc, totals, nested_scope.initializer)?;
                    }
                } else {
                    // It's a struct/class.
                    totals.structs += 1;
                    if nested_scope.initializer != 0 {
                        count_totals_in_scope(ifc, totals, nested_scope.initializer)?;
                    }
                }
            }

            _ => {
                println!("unknown: {:?}", member_decl_index);
                totals.unknown += 1;
            }
        }
    }

    Ok(())
}

#[derive(Default, Debug, Clone)]
struct Totals {
    functions: u64,
    methods: u64,
    namespaces: u64,
    enums: u64,
    typedefs: u64,
    unknown: u64,
    fields: u64,
    templates: u64,
    structs: u64,
    variables: u64,
    intrinsics: u64,
    bitfields: u64,
    explicit_specialization: u64,
    object_macros: u64,
    function_macros: u64,
}
