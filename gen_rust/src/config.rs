//! Loads configuration information from a `modules.toml` file and processes it.
use anyhow::Result;
use serde_derive::Deserialize;
use std::collections::HashMap;

#[cfg(test)]
mod tests;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ModulesConfig {
    pub global_stuff: Option<String>,
    pub header: Vec<Header>,
    pub includes: Option<Vec<String>>,
    pub rustlib: Option<HashMap<String, RustLib>>,
}

/// Describes a C++ "header unit".
///
/// A C++ Header Unit is a binary module that was produced by a conformant C++20 compiler, by
/// consuming a non-modularized C++ header and producing a binary description of it.  The file
/// name is usually `foo.h.ifc`.
#[derive(Deserialize, Clone, Debug)]
pub struct Header {
    pub file: String,
    pub description: Option<String>,
    pub deps: Option<Vec<String>>,
    pub defines: Option<Vec<String>>,
    pub gen_rust_source: Option<bool>,
    pub gen_rust_rlib: Option<bool>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RustLib {
    /// Allows you to override the exact file name, e.g. `libfoo.rlib`.
    /// The default is to construct the rlib filename from the key, so if the key is `awesome`
    /// then the filename will be constructed as `libawesome.rlib`.
    pub file: Option<String>,
    pub gen_cxx_header_unit: Option<bool>,
    pub gen_cxx_module: Option<bool>,
}

pub fn load_config_str(config: &str) -> Result<ModulesConfig> {
    let c: ModulesConfig = toml::from_str(config)?;
    Ok(c)
}
