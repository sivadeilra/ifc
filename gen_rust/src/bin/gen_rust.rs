use anyhow::Result;
use ifc::Ifc;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    /// Primary IFC file to read. This should be a header unit.
    ifc: String,

    /// Zero or more IFC files to reference. These should all be header units.
    ///
    /// Each entry should be in the form `name=path.h.ifc`
    #[structopt(long = "reference")]
    reference: Vec<String>,

    /// Rust source file to write.
    output: String,
}

struct Reference {
    name: String,
    path: String,
    ifc: Ifc,
}

fn main() -> Result<()> {
    let cli_options = Options::from_args();
    let gen_options = gen_rust::Options::default();
    let ifc = Ifc::from_file(std::path::Path::new(&cli_options.ifc))?;

    let mut symbol_map = gen_rust::SymbolMap::default();

    let mut references: Vec<Reference> = Vec::new();

    for ref_ in cli_options.reference {
        if let Some((ifc_name, ifc_path)) = ref_.split_once('=') {
            let ref_data = std::fs::read(ifc_path)?;
            let ref_ifc = Ifc::load(ref_data)?;

            // Read this IFC file and add its symbols to the symbol map.
            let _ref_index = symbol_map.add_ref_ifc(ifc_name, &ref_ifc)?;

            references.push(Reference {
                name: ifc_name.to_string(),
                path: ifc_path.to_string(),
                ifc: ref_ifc,
            });
        } else {
            println!("error: The /reference must have a value in the form /reference:name=path .");
            std::process::exit(1);
        }
    }

    println!("Finished reading referenced IFC files.");
    println!(
        "Total number of symbols found in referenced IFCs: {}",
        symbol_map.map.len()
    );

    let tokens = gen_rust::gen_rust(&ifc, symbol_map, &gen_options)?;

    let output_as_string: String;
    if false {
        output_as_string = tokens.to_string();
    } else {
        println!("Pretty-formatting output");
        let tokens_as_file: syn::File = syn::parse2(tokens)?;

        output_as_string = prettyplease::unparse(&tokens_as_file);
    }

    std::fs::write(&cli_options.output, &output_as_string)?;

    Ok(())
}
