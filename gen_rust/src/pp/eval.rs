#![allow(dead_code)]
#![allow(unused_mut)]
#![allow(unused_variables)]

use super::*;

#[derive(Copy, Clone, Eq, PartialEq)]
enum Op {
    Add,
    Mul,
    Sub,
    Neg,
    Paren,
}

struct EvalContext {
    pub items: Vec<Item>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Value {
    F32(f32),
    F64(f64),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
}

enum Item {
    /// top of stack is lparen for nested expression, but expression is empty
    LParen,
    /// top of stack is binary operator
    Op(Op),
    Value(Value),
}

pub fn eval_macro(ifc: &Ifc, tuple_form: FormIndex) -> Result<Option<Value>> {
    let tuple = ifc.type_tuple().entry(tuple_form.index())?;

    let mut stack: Vec<Item> = Vec::new();

    for i in 0..tuple.cardinality {
        let input_form = *ifc.heap_form().entry(tuple.start + i)?;

        match input_form.tag() {
            FormSort::NUMBER => {
                // let num = ifc.pp_num().entry(form.index())?;
                // let num_string = ifc.get_string(num.spelling)?;
                // output.push_str(num_string);
            }

            FormSort::OPERATOR => {
                let op = ifc.pp_op().entry(input_form.index())?;
                let op_str = ifc.get_string(op.spelling)?;
                debug!("operator: {:?}", op_str);
                match op_str {
                    _ => {
                        warn!("unrecognized op");
                    }
                }
            }

            _ => {
                info!(
                    "cannot evaluate macro, because encountered: {:?}",
                    input_form
                );
                return Ok(None);
            }
        }
    }

    debug!("pretending nothing happened");
    Ok(None)
}
