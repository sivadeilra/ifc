use super::*;

#[test]
fn enums_test() {
    let c = case("enums_test");
    c.compile_cpp("enums_mod.ixx", include_str!("enums_mod.ixx"));

    c.read_ifc_compile_to_rust(&[], "enums_mod.ifc", "enums_mod", Options::default_for_testing());

    c.write_file("checker.rs", include_str!("checker.rs"));

    let mut rustc = c.cmd_rustc();
    rustc.arg("--crate-type=bin");
    rustc.arg("checker.rs");
    c.spawn_and_wait(rustc);

    let checker_path = c.case_tmp_dir.join("checker");
    let checker = c.cmd(checker_path.to_str().unwrap());
    c.spawn_and_wait(checker);
}
