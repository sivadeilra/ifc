use super::*;

#[test]
fn headers_test() {
    let c = case("headers_test");

    c.write_file("foo.h", include_str!("foo.h"));
    c.write_file("bar.h", include_str!("bar.h"));
    // c.write_file("gen.cpp", include_str!("gen.cpp"));

    let mut cl = c.cmd_cl();
    cl.arg("/exportHeader");
    cl.arg("/headerName:quote");
    cl.arg("foo.h");
    cl.arg("bar.h");
    c.spawn_and_wait(cl);

    c.read_ifc_compile_to_rust("bar.h.ifc", "bar_h");

    /*
    c.write_file("main.rs", include_str!("main.rs"));

    let mut rustc = c.cmd_rustc();
    rustc.arg("--crate-type=bin");
    rustc.arg("main.rs");
    c.spawn_and_wait(rustc);

    let main_path = c.case_tmp_dir.join("main");
    let main = c.cmd(main_path.to_str().unwrap());
    c.spawn_and_wait(main);
    */
}
