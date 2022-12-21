use super::*;

fn load_config(s: &str) -> ModulesConfig {
    match load_config_str(s) {
        Ok(c) => {
            println!("Loaded config:\n{:#?}", c);
            c
        }
        Err(e) => {
            panic!("Failed to load config: {:?}", e);
        }
    }
}

#[test]
fn basic() {
    load_config(
        r#"

global_stuff = "blah blah"

includes = [
    "$(BASEDIR)/onecoreuap/windows/core/ntgdi/inc",
    "$(OBJECT_ROOT)/onecoreuap/windows/core/ntgdi/w32inc/$(O)",
]


[[header]]
file = "windows.h"
description = "Giant public Windows SDK header"
deps = ["nt.h"]
defines = ["WIN32_LEAN_AND_MEAN"]
gen-rust-rlib = true


[[header]]
file = "nt.h"
description = "Windows NT definitions"


[rustlib.truetype]
gen-cxx-header-unit = true
gen-cxx-module = true


    "#,
    );
}
