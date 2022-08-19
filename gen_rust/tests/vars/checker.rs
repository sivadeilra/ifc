extern crate vars_mod;
use vars_mod::*;

fn main() {
    // The main thing we are testing is that the constants have the expected type.
    assert_eq!(CONSTEXPR_VAR_WCHAR_T, 65u16);
    assert_eq!(CONSTEXPR_VAR_CHAR, 123i8);
    assert_eq!(CONSTEXPR_VAR_SCHAR, 123i8);
    assert_eq!(CONSTEXPR_VAR_UCHAR, 123u8);
    assert_eq!(CONSTEXPR_VAR_SHORT, 123i16);
    assert_eq!(CONSTEXPR_VAR_INT, 123i32);
    assert_eq!(CONSTEXPR_VAR_LONG, 123i32);
    assert_eq!(CONSTEXPR_VAR_LONGLONG, 123i64);
    assert_eq!(CONSTEXPR_VAR_INT8, 123i8);
    assert_eq!(CONSTEXPR_VAR_INT16, 123i16);
    assert_eq!(CONSTEXPR_VAR_INT32, 123i32);
    assert_eq!(CONSTEXPR_VAR_INT64, 123i64);
    assert_eq!(CONSTEXPR_VAR_USHORT, 123u16);
    assert_eq!(CONSTEXPR_VAR_UINT, 123u32);
    assert_eq!(CONSTEXPR_VAR_ULONG, 123u32);
    assert_eq!(CONSTEXPR_VAR_ULONGLONG, 123u64);
    assert_eq!(CONSTEXPR_VAR_UINT8, 123u8);
    assert_eq!(CONSTEXPR_VAR_UINT16, 123u16);
    assert_eq!(CONSTEXPR_VAR_UINT32, 123u32);
    assert_eq!(CONSTEXPR_VAR_UINT64, 123u64);
}
