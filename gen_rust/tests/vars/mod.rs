use super::*;

#[test]
fn vars_test() {
    let c = case("vars_test");
    c.compile_cpp("vars_mod.ixx", include_str!("vars_mod.ixx"));

    c.read_ifc_compile_to_rust("vars_mod.ifc", "vars_mod");

    c.write_file("main.rs", include_str!("main.rs"));

    let mut rustc = c.cmd_rustc();
    rustc.arg("--crate-type=bin");
    rustc.arg("main.rs");
    c.spawn_and_wait(rustc);

    let main_path = c.case_tmp_dir.join("main");
    let main = c.cmd(main_path.to_str().unwrap());
    c.spawn_and_wait(main);
}
