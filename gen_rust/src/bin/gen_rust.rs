use anyhow::Result;
use ifc::Ifc;
use structopt::StructOpt;

#[derive(StructOpt)]
struct Options {
    /// IFC file to read.
    ifc: String,

    /// Rust file to write
    output: String,
}

fn main() -> Result<()> {
    let cli_options = Options::from_args();
    let gen_options = gen_rust::Options::default();
    let ifc = Ifc::from_file(std::path::Path::new(&cli_options.ifc))?;

    let tokens = gen_rust::gen_rust(&ifc, &gen_options)?;

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
