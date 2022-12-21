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
    c.spawn_and_wait(cl);

    let mut cl = c.cmd_cl();
    cl.arg("/translateInclude");
    cl.arg("/headerUnit:quote foo.h=foo.h.ifc");
    cl.arg("/exportHeader");
    cl.arg("/headerName:quote");
    cl.arg("bar.h");
    c.spawn_and_wait(cl);

    c.read_ifc_compile_to_rust(&[], "foo.h.ifc", "foo", Options::for_testing(&TestOptions{ blocklist_macro: &["FOO_DECREMENT"], ..Default::default() }));

    c.read_ifc_compile_to_rust(&[("foo", "foo.h.ifc")], "bar.h.ifc", "bar", Options::for_testing(&TestOptions{ blocklist_macro: &["FOO_DECREMENT"], ..Default::default() }));

    c.write_file("checker.rs", include_str!("checker.rs"));

    let mut rustc = c.cmd_rustc();
    rustc.arg("--crate-type=bin");
    rustc.arg("checker.rs");
    c.spawn_and_wait(rustc);

    let checker_path = c.case_tmp_dir.join("checker");
    let checker = c.cmd(checker_path.to_str().unwrap());
    c.spawn_and_wait(checker);
}
