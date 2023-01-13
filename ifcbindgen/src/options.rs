use anyhow::{Context, Result};
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;

fn parse_ifc_reference(src: &str) -> Result<(String, PathBuf)> {
    let (name, path) = src
        .split_once('=')
        .context("Argument's value must be in the format `crate_name=ifc_path`")?;
    Ok((
        name.to_string(),
        PathBuf::from_str(path).context("IFC's path contains invalid characters")?,
    ))
}

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Filename to read. This is usually `<something>.ifc`.
    #[structopt(short, long, parse(from_os_str))]
    pub ifc: PathBuf,

    /// Filename to output. This is usually `<something>.rs`.
    #[structopt(short, long, parse(from_os_str))]
    pub output: PathBuf,

    /// References to other IFC files.
    /// Must be in the format `crate_name=ifc_path`
    #[structopt(long, parse(try_from_str = parse_ifc_reference))]
    pub references: Vec<(String, PathBuf)>,

    /// Output verbosity.
    /// Default: errors.
    /// -v: warnings.
    /// -vv: info.
    /// -vvv: debug.
    /// -vvvv: trace.
    #[structopt(short, parse(from_occurrences))]
    pub verbosity: u8,

    #[structopt(flatten)]
    pub gen_options: gen_rust::Options,
}