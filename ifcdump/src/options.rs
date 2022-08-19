use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Options {
    /// Filename to read. This is usually `<something>.ifc`.
    pub ifc: String,

    /// Show everything possible.
    #[structopt(short = "a", long = "all")]
    pub all: bool,

    /// Show all of the source files (headers, usually) that contributed to this IFC.
    #[structopt(long)]
    pub sources: bool,

    /// Show preprocessor state, i.e. `#define ...`
    #[structopt(short = "d", long = "defines")]
    pub defines: bool,

    /// Show function definitions.
    #[structopt(long = "functions")]
    pub functions: bool,

    /// Show all types (structs, enums, typedefs, fundamental types).
    #[structopt(long = "types")]
    pub types: bool,

    /// Show all enum definitions.
    #[structopt(long = "enums")]
    pub enums: bool,

    /// Show structure definitions, but not all types (do not show typedefs).
    #[structopt(long = "structs")]
    pub structs: bool,

    /// Show `typedef` definitions.
    #[structopt(long = "typedefs")]
    pub typedefs: bool,

    /// Show fundamental types.
    #[structopt(long = "funtypes")]
    pub funtypes: bool,

    /// Show the partitions (tables).
    #[structopt(long = "parts")]
    pub parts: bool,

    /// A filter (regex) to apply to things being dump.
    #[structopt(long = "where")]
    pub where_: Option<String>,

    /// Specifies that the `--where` parameter is case-sensitive. By default, the `--where`
    /// parameter is not case-sensitive.
    #[structopt(long = "wcase")]
    pub wcase: bool,

    #[structopt(long = "summary")]
    pub summary: bool,

    #[structopt(long = "verbose")]
    pub verbose: bool,

    /// Maximum number of results to print.
    #[structopt(long = "max", default_value = "1000000")]
    pub max_results: u32,
}
