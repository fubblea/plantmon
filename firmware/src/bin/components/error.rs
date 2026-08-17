use alloc::string::String;

#[derive(Debug)]
pub enum ComponentError {
    ReadError(String),
    AdcConversionError(String),
}
