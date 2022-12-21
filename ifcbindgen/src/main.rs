#![forbid(unused_must_use)]
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use ifc::*;
use structopt::StructOpt;

mod options;

fn read_ifc(ifc_file_path: &std::path::PathBuf) -> Ifc {
    let ifc_data = std::fs::read(ifc_file_path).expect("failed to read IFC file");
    Ifc::load(ifc_data).expect("failed to parse IFC file data")
}

fn main() -> Result<()> {
    let options = options::Options::from_args();
    let filter_level = match options.verbosity {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        _ => "trace",
    };
    let env = env_logger::Env::default().default_filter_or(filter_level);
    env_logger::init_from_env(env);

    let ifc = read_ifc(&std::fs::canonicalize(&options.ifc).context("Invalid path to the ifc")?);

    let mut symbol_map = gen_rust::SymbolMap::default();
    for (ref_name, ref_filename) in options.references.iter() {
        let ref_path = std::fs::canonicalize(ref_filename).context("Invalid path to referenced IFC")?;
        let ref_ifc = read_ifc(&ref_path);
        symbol_map.add_ref_ifc(ref_name, &ref_ifc).context("Failed to add reference to IFC")?;
    }

    let rust_generated_code =
        gen_rust::gen_rust(&ifc, symbol_map, &options.gen_options)?;
    let rust_tokens_as_file: syn::File = syn::parse2(rust_generated_code)
        .context("Could not parse generated Rust tokens")?;
    let rust_output_as_string = prettyplease::unparse(&rust_tokens_as_file);

    let mut output_path = std::env::current_dir().context("No current directory?")?;
    output_path.push(options.output);
    std::fs::write(
        output_path,
        &rust_output_as_string,
    )
    .context("Failed to write output file")?;

    Ok(())
}
