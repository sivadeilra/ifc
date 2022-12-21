use super::*;

#[static_init::dynamic]
static INIT_LOGGER: () = env_logger::builder().format_timestamp(None).init();

// this sorta works
// #[cfg(nope)]
#[test]
#[ignore]
fn more_stuff() {
    let mut gen = Gen::default();

    writestr!(
        gen.output.header,
        r"// This is generated code. Do not edit.

#pragma once
"
    );

    let sysroot_lib = r"c:\users\ardavis\.rustup\toolchains\nightly-2022-08-08-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\lib";

    let opts = vec![
        "rustc".to_string(),
        "--crate-type=rlib".to_string(),
        "--emit=metadata".to_string(),
        "--crate-name=hello".to_string(),
        "--target=x86_64-pc-windows-msvc".to_string(),
        "-L".to_string(),
        sysroot_lib.to_string(),
        r"--out-dir=d:\rmeta_reader\out".to_string(),
        r"d:\rmeta_reader\inputs\hello.rs".to_string(),
    ];

    let rc = rustc_driver::RunCompiler::new(&opts, &mut gen);
    rc.run().unwrap();

    info!("done");

    std::fs::write(r"d:\temp\output.h", &gen.output.header).unwrap();
}

#[cfg(nope)]
#[test]
fn direct_rmeta_reader() {
    let session_globals =
        rustc_span::SessionGlobals::new(rustc_span::edition::Edition::Edition2018);

    info!("setting session globals");
    rustc_span::set_session_globals_then(&session_globals, || {
        let sopts: config::Options = Default::default();
        let local_crate_source_file: Option<PathBuf> = None;
        let bundle = None; // : Option<Lrc<rustc_errors::FluentBundle>> = None;
        let registry = Registry::new(&[]);
        let diagnostics_output: DiagnosticOutput = DiagnosticOutput::Default;
        let driver_lint_caps = Default::default(); // : FxHashMap<lint::LintId, lint::Level> = Default::default();
        let file_loader: Option<Box<dyn FileLoader + Send + Sync + 'static>> = None;
        let target_override = None; // : Option<Target> = None;

        info!("calling build_session");
        let session: Session = rustc_session::build_session(
            sopts,
            local_crate_source_file,
            bundle,
            registry,
            diagnostics_output,
            driver_lint_caps,
            file_loader,
            target_override,
        );

        let crate_name = "not_a_real_crate";

        let metadata_loader: Box<rustc_session::cstore::MetadataLoaderDyn> =
            Box::new(rustc_codegen_ssa::back::metadata::DefaultMetadataLoader);

        let target_triple: TargetTriple =
            TargetTriple::TargetTriple("x86_64-pc-windows-msvc".to_string());
        let target: Target = Target::expect_builtin(&target_triple);

        let md = metadata_loader
            .get_rlib_metadata(
                &target,
                std::path::Path::new(r"d:\rmeta_reader\libhello.rlib"),
            )
            .unwrap();

        info!("Loaded rmeta, len = {}", md.len());

        if true {
            let krate = ast::Crate {
                attrs: Default::default(),
                items: Default::default(),
                spans: Default::default(),
                id: ast::CRATE_NODE_ID,
                is_placeholder: false,
            };

            rustc_interface::register_plugins(
                &session,
                &metadata_loader,
                |session, lint_store| {
                    info!("register_lints getting called");
                },
                krate,
                crate_name,
            );

            info!("creating crate loader");
            let mut loader: CrateLoader = CrateLoader::new(&session, metadata_loader, crate_name);
            info!("created crate loader");

            let extern_crate_name: Symbol = Symbol::intern("std");
            let extern_crate_span: Span = Span::default();
            let crate_num = loader.process_path_extern(extern_crate_name, extern_crate_span);
            info!("crate_num = {:?}", crate_num);
        }
    });

    info!("done");
}

#[cfg(nope)]
#[test]
fn stuff() {
    /*
    let sopts: config::Options = Default::default();
    let local_crate_source_file: Option<PathBuf> = None;
    let bundle = None; // : Option<Lrc<rustc_errors::FluentBundle>> = None;
    let registry = Registry::new(&[]);
    let diagnostics_output: DiagnosticOutput = DiagnosticOutput::Default;
    let driver_lint_caps = Default::default(); // : FxHashMap<lint::LintId, lint::Level> = Default::default();
    let file_loader: Option<Box<dyn FileLoader + Send + Sync + 'static>> = None;
    let target_override = None; // : Option<Target> = None;

    let session: Session = rustc_session::build_session(
        sopts,
        local_crate_source_file,
        bundle,
        registry,
        diagnostics_output,
        driver_lint_caps,
        file_loader,
        target_override,
    );

    let loader: CrateLoader;
    */

    let session_globals =
        rustc_span::SessionGlobals::new(rustc_span::edition::Edition::Edition2018);

    info!("setting session globals");
    rustc_span::set_session_globals_then(&session_globals, || {
        info!("creating config");

        let config: rustc_interface::Config = rustc_interface::Config {
            /// Command line options
            opts: Default::default(), // config::Options,

            /// cfg! configuration in addition to the default ones
            crate_cfg: Default::default(), // FxHashSet<(String, Option<String>)>,
            crate_check_cfg: Default::default(), // CheckCfg::default(),

            input: rustc_session::config::Input::File(PathBuf::from(
                r"d:\rmeta_reader\input\hello.rs",
            )),
            input_path: Some(PathBuf::from(r"d:\rmeta_reader\input")), // Option<PathBuf>,
            output_dir: Some(PathBuf::from(r"d:\rmeta_reader\out")),   // Option<PathBuf>,
            output_file: None,                                         // Option<PathBuf>,
            file_loader: None, // Option<Box<dyn FileLoader + Send + Sync>>,
            diagnostic_output: DiagnosticOutput::Default,

            lint_caps: Default::default(), // FxHashMap<lint::LintId, lint::Level>,

            /// This is a callback from the driver that is called when [`ParseSess`] is created.
            parse_sess_created: None, // Option<Box<dyn FnOnce(&mut ParseSess) + Send>>,

            /// This is a callback from the driver that is called when we're registering lints;
            /// it is called during plugin registration when we have the LintStore in a non-shared state.
            ///
            /// Note that if you find a Some here you probably want to call that function in the new
            /// function being registered.
            register_lints: None, // Option<Box<dyn Fn(&Session, &mut LintStore) + Send + Sync>>,

            /// This is a callback from the driver that is called just after we have populated
            /// the list of queries.
            ///
            /// The second parameter is local providers and the third parameter is external providers.
            override_queries: None, // Option<fn(&Session, &mut ty::query::Providers, &mut ty::query::ExternProviders)>,

            /// This is a callback from the driver that is called to create a codegen backend.
            make_codegen_backend: None, // Option<Box<dyn FnOnce(&config::Options) -> Box<dyn CodegenBackend> + Send>>,

            /// Registry of diagnostics codes.
            registry: Registry::new(&[]),
        };

        info!("calling create_compiler_and_run");
        rustc_interface::interface::create_compiler_and_run(
            config,
            |c: &rustc_interface::interface::Compiler| {
                info!("got called back with compiler");

                let session = c.session();
            },
        );

        info!("done.");
    });
}
