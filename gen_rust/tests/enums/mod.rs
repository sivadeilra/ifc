use super::*;

#[test]
fn enums_test() {
    let c = case("enums_test");
    c.compile_cpp("enums_mod.ixx", include_str!("enums_mod.ixx"));

    c.read_ifc_compile_to_rust("enums_mod.ifc", "enums_mod");

    c.write_file("main.rs", include_str!("main.rs"));

    let mut rustc = c.cmd_rustc();
    rustc.arg("--crate-type=bin");
    rustc.arg("main.rs");
    c.spawn_and_wait(rustc);

    let main_path = c.case_tmp_dir.join("main");
    let main = c.cmd(main_path.to_str().unwrap());
    c.spawn_and_wait(main);
}
