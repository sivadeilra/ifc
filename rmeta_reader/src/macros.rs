macro_rules! writestr {
    ($s:expr, $($t:tt)*) => {
        {
            let s: &mut String = &mut $s;
            write!(s, $($t)*).unwrap();
        }
    }
}

macro_rules! writelnstr {
    ($s:expr, $($t:tt)*) => {
        {
            let s: &mut String = &mut $s;
            writeln!(s, $($t)*).unwrap();
        }
    }
}
