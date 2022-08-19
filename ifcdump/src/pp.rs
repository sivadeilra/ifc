use super::*;
use regex::Regex;

pub fn dump_pp(ifc: &Ifc, matcher: &mut dyn FnMut(&str) -> bool) -> Result<()> {
    let mut s = String::new();

    let mut param_names: Vec<String> = Vec::new();

    println!("Function-like macros:");
    println!();
    for func_like in ifc.macro_function_like().entries.iter() {
        let name = ifc.get_string(func_like.name)?;
        if !matcher(name) {
            continue;
        }

        let arity = func_like.arity();

        // Set up param name vector.  We reuse the vector. Clear any existing strings, so we don't
        // accidentally reuse their contents.  Add new strings, if necessary.
        let param_arity = arity as usize;
        while param_names.len() < param_arity {
            param_names.push(String::new());
        }
        for s in param_names.iter_mut() {
            s.clear();
        }

        if arity != 0 {
            if func_like.parameters.tag() == FormSort::TUPLE {
                let params_tuple = ifc.pp_tuple().entry(func_like.parameters.index())?;
                if params_tuple.cardinality == func_like.arity() {
                    for i in 0..param_arity {
                        let param_str = &mut param_names[i];
                        let param_form = *ifc.heap_form().entry(params_tuple.start + i as u32)?;
                        if param_form.tag() == FormSort::PARAMETER {
                            let p = ifc.pp_param().entry(param_form.index())?;
                            let s = ifc.get_string(p.spelling)?;
                            param_str.push_str(s);
                        } else {
                            println!("parameter had wrong sort: {:?}", param_form);
                            param_str.push_str("__bad__");
                        }
                    }
                } else {
                    println!("warning: tuple arity does not match parameters arity");
                }
            }
        }

        form_to_string(ifc, func_like.body, &mut s)?;
        println!(
            "#define {}({}) {}",
            name,
            param_names[..param_arity].join(", ").to_string(),
            s
        );
    }
    println!();

    println!("Object-like macros:");
    println!();
    for object in ifc.macro_object_like().entries.iter() {
        let name = ifc.get_string(object.name)?;
        if !matcher(name) {
            continue;
        }

        form_to_string(ifc, object.body, &mut s)?;
        println!("#define {} {}", name, s);
    }
    println!();

    Ok(())
}

fn form_to_string(ifc: &Ifc, form: FormIndex, output: &mut String) -> Result<()> {
    output.clear();
    form_to_string_rec(ifc, form, output)
}

fn form_to_string_rec(ifc: &Ifc, form: FormIndex, output: &mut String) -> Result<()> {
    use core::fmt::Write;

    match form.tag() {
        FormSort::IDENTIFIER => {
            let id = ifc.pp_ident().entry(form.index())?;
            let id_string = ifc.get_string(id.spelling)?;
            output.push_str(" ");
            output.push_str(&id_string);
        }

        FormSort::PARENTHESIZED => {
            let paren = ifc.pp_paren().entry(form.index())?;
            output.push_str("(");
            form_to_string_rec(ifc, paren.operand, output)?;
            output.push_str(")");
        }

        FormSort::TUPLE => {
            let tuple = ifc.pp_tuple().entry(form.index())?;
            for i in 0..tuple.cardinality {
                let element_form = *ifc.heap_form().entry(tuple.start + i)?;
                form_to_string_rec(ifc, element_form, output)?;
            }
        }

        FormSort::NUMBER => {
            let num = ifc.pp_num().entry(form.index())?;
            let num_string = ifc.get_string(num.spelling)?;
            output.push_str(num_string);
        }

        FormSort::OPERATOR => {
            let op = ifc.pp_op().entry(form.index())?;
            let op_string = ifc.get_string(op.spelling)?;
            output.push_str(op_string);
        }

        FormSort::PARAMETER => {
            let param = ifc.pp_param().entry(form.index())?;
            let param_string = ifc.get_string(param.spelling)?;
            output.push_str(param_string);
        }

        FormSort::STRINGIZE => {
            let param = ifc.pp_stringize().entry(form.index())?;
            output.push_str(" #");
            form_to_string_rec(ifc, param.operand, output)?;
        }

        FormSort::STRING => {
            let param = ifc.pp_string().entry(form.index())?;
            output.push('(');
            let s = ifc.get_string(param.spelling)?;
            output.push(')');
        }

        FormSort::CATENATE => {
            let cat = ifc.pp_catenate().entry(form.index())?;
            form_to_string_rec(ifc, cat.first, output)?;
            output.push_str(" ## ");
            form_to_string_rec(ifc, cat.second, output)?;
        }

        FormSort::PRAGMA => {
            let pragma = ifc.pp_pragma().entry(form.index())?;
            output.push_str(" _Pragma(");
            form_to_string_rec(ifc, pragma.operand, output)?;
            output.push_str(") ");
        }

        _ => {
            write!(output, "???{:?}", form)?;
        }
    }

    Ok(())
}
