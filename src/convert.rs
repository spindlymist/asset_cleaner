mod c {
    #[repr(C)]
    pub union IntOrDouble {
        pub int: i32,
        pub double: f64,
    }
    
    #[repr(C)]
    pub struct MmfNumber {
        pub value: IntOrDouble,
        pub is_double: bool,
    }
    
    unsafe extern "C" {
        pub fn mmf_double_to_str(
            value: std::os::raw::c_double,
            buffer: *mut std::os::raw::c_char,
            buffer_size: usize,
        ) -> usize;
        pub fn mmf_parse_number(s: *const std::os::raw::c_char) -> MmfNumber;
        pub fn ini_parse_number(s: *const std::os::raw::c_char) -> MmfNumber;
    }
}

#[derive(Debug, PartialEq)]
pub enum Number {
    Int(i32),
    Double(f64),
}

impl From<c::MmfNumber> for Number {
    fn from(num: c::MmfNumber) -> Self {
        if num.is_double {
            Self::Double(unsafe { num.value.double })
        }
        else {
            Self::Int(unsafe { num.value.int })
        }
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(val) => write!(f, "{val}"),
            Self::Double(val) => write!(f, "{}", mmf_double_to_str(*val))
        }
    }
}

pub fn mmf_double_to_str(value: f64) -> String {
    let (buffer, _, capacity) = String::with_capacity(16).into_raw_parts();
    unsafe {
        let length = c::mmf_double_to_str(value, buffer as *mut i8, capacity);
        String::from_raw_parts(buffer, length, capacity)
    }
}

pub fn mmf_parse_number(s: &str) -> Number {
    let cstring = std::ffi::CString::new(s).unwrap();
    let number = unsafe { c::mmf_parse_number(cstring.as_ptr()) };
    Number::from(number)
}

pub fn ini_parse_number(s: &str) -> Number {
    let cstring = std::ffi::CString::new(s).unwrap();
    let number = unsafe { c::ini_parse_number(cstring.as_ptr()) };
    Number::from(number)
}

#[cfg(test)]
mod test {
    use super::*;
    
    #[test]
    fn mmf_double_to_str_works() {
        for (input, expected) in [
            (          999.001, "999.001"),
            (         999.0001, "999"),
            (          1000.01, "1000.01"),
            (         1000.001, "1000"),
            (         999999.0, "999999"),
            (        1000000.0, "1e+006"),
            (        1000001.0, "1e+006"),
            (        1000010.0, "1.00001e+006"),
            (        1000100.0, "1.0001e+006"),
            (        -999999.0, "-999999"),
            (       -1000000.0, "-1e+006"),
            (     2147483647.0, "2.14748e+009"),
            (    f64::INFINITY, "1.#INF"),
            (f64::NEG_INFINITY, "-1.#INF"),
            (         f64::NAN, "-1.#IND")
        ] {
            let actual = mmf_double_to_str(input);
            assert_eq!(actual, expected);
        }
    }
    
    #[test]
    fn mmf_parse_number_works() {
        for (input, expected) in [
            // Invalid
            ("",            Number::Int(0)),
            ("+",           Number::Int(0)),
            ("-",           Number::Int(0)),
            (".",           Number::Int(0)),
            ("-.",          Number::Double(0.0)),
            ("-e",          Number::Double(0.0)),
            ("e3",          Number::Int(0)),
            ("knytt",       Number::Int(0)),
            // Integers
            ("-0",          Number::Int(0)),
            ("123",         Number::Int(123)),
            ("+123",        Number::Int(0)),
            ("-123",        Number::Int(-123)),
            ("123knytt",    Number::Int(123)),
            ("123knytt.",   Number::Int(123)),
            // Max integer range
            ("2147483646",  Number::Int(2147483646)),
            ("2147483647",  Number::Double(2147483647.0)),
            ("2147483648",  Number::Double(2147483648.0)),
            // Min integer range
            ("-2147483646", Number::Int(-2147483646)),
            ("-2147483647", Number::Double(-2147483647.0)),
            ("-2147483648", Number::Double(-2147483648.0)),
            ("-2147483649", Number::Double(-2147483649.0)),
            // Doubles
            ("-0.0",        Number::Double(-0.0)),
            ("123.456",     Number::Double(123.456)),
            ("123.",        Number::Double(123.0)),
            (".456",        Number::Int(0)),
            ("+123.456",    Number::Int(0)),
            ("+123.",       Number::Int(0)),
            ("+.456",       Number::Int(0)),
            ("-123.456",    Number::Double(-123.456)),
            ("-123.",       Number::Double(-123.0)),
            ("-.456",       Number::Double(-0.456)),
            // Exponents
            ("123.456e",    Number::Double(123.456)),
            ("123.e",       Number::Double(123.0)),
            ("123e",        Number::Double(123.0)),
            ("123.456E",    Number::Double(123.456)),
            ("123.E",       Number::Double(123.0)),
            ("123E",        Number::Double(123.0)),
            ("123.456e3",   Number::Double(123456.0)),
            ("123.e3",      Number::Double(123000.0)),
            ("123e3",       Number::Double(123000.0)),
            ("123.456e+3",  Number::Double(123456.0)),
            ("123.456e-3",  Number::Double(0.123456)),
        ] {
            let actual = mmf_parse_number(input);
            assert_eq!(Number::from(actual), expected, "while parsing {input}");
        }
    }
    
    #[test]
    fn ini_parse_number_works() {        
        for (input, expected) in [
            // Invalid
            ("",            Number::Int(0)),
            ("+",           Number::Int(0)),
            ("-",           Number::Int(0)),
            (".",           Number::Double(0.0)),
            ("-.",          Number::Double(0.0)),
            ("-e",          Number::Int(0)),
            ("e3",          Number::Int(0)),
            ("knytt",       Number::Int(0)),
            // Integers
            ("-0",          Number::Int(0)),
            ("123",         Number::Int(123)),
            ("+123",        Number::Int(123)),
            ("-123",        Number::Int(-123)),
            ("123knytt",    Number::Int(123)),
            ("123knytt.",   Number::Double(123.0)),
            // Max integer range
            ("2147483647",  Number::Int(2147483647)),
            // Min integer range
            ("-2147483647", Number::Int(-2147483647)),
            ("-2147483648", Number::Int(-2147483648)),
            // Doubles
            ("-0.0",        Number::Double(-0.0)),
            ("123.456",     Number::Double(f64::from(123.456f32))),
            ("123.",        Number::Double(123.0)),
            (".456",        Number::Double(f64::from(0.456f32))),
            ("+123.456",    Number::Double(f64::from(123.456f32))),
            ("+123.",       Number::Double(123.0)),
            ("+.456",       Number::Double(f64::from(0.456f32))),
            ("-123.456",    Number::Double(f64::from(-123.456f32))),
            ("-123.",       Number::Double(-123.0)),
            ("-.456",       Number::Double(f64::from(-0.456f32))),
            // Exponents
            ("123.456e",    Number::Double(f64::from(123.456f32))),
            ("123.e",       Number::Double(123.0)),
            ("123e",        Number::Int(123)),
            ("123.456E",    Number::Double(f64::from(123.456f32))),
            ("123.E",       Number::Double(123.0)),
            ("123E",        Number::Int(123)),
            ("123.456e3",   Number::Double(123456.0)),
            ("123.e3",      Number::Double(123000.0)),
            ("123e3",       Number::Int(123)),
            ("123.456e+3",  Number::Double(123456.0)),
            ("123.456e-3",  Number::Double(f64::from(0.123456f32))),
        ] {
            let actual = ini_parse_number(input);
            assert_eq!(Number::from(actual), expected, "while parsing {input}");
        }
    }
}
