extern crate enums_mod;
use enums_mod::*;

fn main() {
    assert_eq!(SIMPLE_A.0, 10);
    assert_eq!(SIMPLE_B.0, 20);
    assert_eq!(SIMPLE_A, SIMPLE_A);
    println!("SIMPLE_A = {:?}", get_a());

    // Check FancyEnum
    let _value: i64 = FancyEnum::Red.0;
    println!("FancyEnum::Red = {:?}", FancyEnum::Red);

    assert_eq!(SIMPLE_NEGATIVE.0, -50);
}

fn get_a() -> SIMPLE_ENUM {
    SIMPLE_A
}
