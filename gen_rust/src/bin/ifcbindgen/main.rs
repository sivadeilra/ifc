use std::{io::Write, sync::atomic::AtomicBool};

use anyhow::{bail, Context, Result};
use gen_rust::log_error;
use ifc::*;
use structopt::StructOpt;

mod options;

static HAS_SEEN_ERROR: AtomicBool = AtomicBool::new(false);

fn read_ifc(ifc_file_path: &std::path::PathBuf) -> Ifc {
    let ifc_data = std::fs::read(ifc_file_path).expect("failed to read IFC file");
    Ifc::load(ifc_data).expect("failed to parse IFC file data")
}

fn main() -> Result<()> {
    let options = options::Options::from_args();
    let filter_level = match options.verbosity {
        0 => log::LevelFilter::Error,
        1 => log::LevelFilter::Warn,
        2 => log::LevelFilter::Info,
        3 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };

    // Use a build.exe compatible logging format:
    // <tool name> : <level> : <message>
    let mut builder = env_logger::builder();
    builder
        .format(|buf, record| writeln!(buf, "ifcbindgen : {} : {}", record.level(), record.args()))
        .filter_level(filter_level);
    log::set_boxed_logger(Box::new(ErrorMonitoringLogger {
        set_on_error: &HAS_SEEN_ERROR,
        forward_to: builder.build(),
    }))
    .with_context(|| "Setting the logger failed")?;
    log::set_max_level(filter_level);

    log_error! { {
        let ifc = read_ifc(&std::fs::canonicalize(&options.ifc).context("Invalid path to the ifc")?);

        let mut symbol_map = gen_rust::SymbolMap::default();
        for (ref_name, ref_filename) in options.references {
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
    } -> (), "ifcbindgen failed" };

    if HAS_SEEN_ERROR.load(std::sync::atomic::Ordering::Relaxed) {
        bail!("One or more errors occured");
    }

    Ok(())
}

struct ErrorMonitoringLogger<T: log::Log> {
    set_on_error: &'static AtomicBool,
    forward_to: T,
}

impl<T: log::Log> log::Log for ErrorMonitoringLogger<T> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.forward_to.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if record.level() <= log::Level::Error {
            self.set_on_error
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.forward_to.log(record)
    }

    fn flush(&self) {
        self.forward_to.flush()
    }
}
