#![forbid(unused_must_use)]

use gen_rust::{ Options, TestOptions };
use ifc::Ifc;
use std::path::{Path, PathBuf};
use std::process::Command;

mod enums;
mod headers;
mod vars;

// It's a bit strange that we read this environment variable at compilation time.
const CARGO_TARGET_TMPDIR: &str = env!("CARGO_TARGET_TMPDIR");

struct Case {
    case_tmp_dir: PathBuf,
}

fn case(case_name: &str) -> Case {
    let case_tmp_dir = Path::new(CARGO_TARGET_TMPDIR)
        .join("gen_rust_tests")
        .join(case_name);

    println!("----- case: {} -----", case_name);
    println!("case_tmp_dir: {}", case_tmp_dir.display());

    // create_dir_all explicitly guarantees that it is safe to race this.
    std::fs::create_dir_all(&case_tmp_dir).unwrap();

    Case { case_tmp_dir }
}

impl Case {
    fn cmd(&self, exe: &str) -> Command {
        use std::process::Stdio;
        let mut cmd = Command::new(exe);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        cmd.current_dir(&self.case_tmp_dir);
        cmd
    }

    fn cmd_cl(&self) -> Command {
        let mut cl = self.cmd("cl.exe");
        cl.arg("/nologo");
        cl.arg("/std:c++20");
        cl.arg("/c");
        cl
    }

    fn cmd_rustc(&self) -> Command {
        let mut c = self.cmd("rustc");
        c.env("RUSTC_BOOTSTRAP", "1");
        c.arg("--edition=2018");
        c.arg("-Zmacro-backtrace");
        c.arg("-L");
        c.arg(".");
        c
    }

    fn spawn_and_wait(&self, mut cmd: Command) {
        println!("Spawning process: {:?}", cmd);
        let mut child = cmd.spawn().expect("Failed to spawn child process");
        let exit = child.wait().expect("Failed to wait for child process");
        assert!(exit.success(), "Child process failed");
        println!("child process succeeded");
    }

    fn compile_cpp(&self, cpp_source_file_name: &str, cpp_source_code: &str) {
        println!("cpp_source_file_name: {}", cpp_source_file_name);

        let cpp_source_file_path = self.case_tmp_dir.join(&cpp_source_file_name);
        std::fs::write(&cpp_source_file_path, cpp_source_code)
            .expect("Expected to write C++ source file");

        println!("invoking C++ compiler");
        let mut cl = self.cmd_cl();
        cl.arg(&cpp_source_file_name);

        self.spawn_and_wait(cl);
    }

    fn read_ifc(&self, ifc_filename: &str) -> Ifc {
        let ifc_file_path = self.case_tmp_dir.join(ifc_filename);
        println!("ifc_file_path: {}", ifc_file_path.display());
        let ifc_data = std::fs::read(&ifc_file_path).expect("failed to read IFC file");
        let ifc = Ifc::load(ifc_data).expect("failed to parse IFC file data");
        ifc
    }

    fn write_file(&self, filename: &str, contents: &str) {
        let path = self.case_tmp_dir.join(filename);
        println!("writing file: {}", path.display());

        std::fs::write(&path, contents).expect("Expected to write file");
    }

    fn read_ifc_compile_to_rust(
        &self,
        ifc_references: &[(&str, &str)],
        ifc_filename: &str,
        rust_crate_name: &str,
        ifc_options: Options,
    ) {
        let ifc = self.read_ifc(ifc_filename);

        let rust_source_file_name = format!("{}.rs", rust_crate_name);
        let rust_source_path = self.case_tmp_dir.join(&rust_source_file_name);

        let mut symbol_map = gen_rust::SymbolMap::default();
        for (ref_name, ref_filename) in ifc_references.iter() {
            let ref_path = self.case_tmp_dir.join(ref_filename);
            let ref_ifc = self.read_ifc(&ref_path.to_string_lossy());
            symbol_map.add_ref_ifc(ref_name, &ref_ifc).unwrap();
        }

        let rust_generated_code = gen_rust::gen_rust(&ifc, symbol_map, &ifc_options)
            .expect("Expected gen_rust to succeed");
        let rust_tokens_as_file: syn::File = syn::parse2(rust_generated_code)
            .expect("Expected gen_rust to generate well-formed Rust tokens");
        let rust_output_as_string = prettyplease::unparse(&rust_tokens_as_file);

        std::fs::write(&rust_source_path, &rust_output_as_string)
            .expect("Expected to write Rust source code");

        let mut rustc = self.cmd_rustc();
        rustc.arg("--crate-type=rlib");
        rustc.arg(&rust_source_path);
        self.spawn_and_wait(rustc);
        println!("Compiled IFC-to-Rust crate.");
    }
}

#[static_init::dynamic]
static TEST_LOGGER: () = {
    env_logger::init();
};
