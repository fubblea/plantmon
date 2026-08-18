use alloc::string::String;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComponentError {
    #[error("Read error: {0}")]
    Read(String),
    #[error("ADC conversion error: {0}")]
    AdcConversion(String),
    #[error("Expected value of type {0}, but got {1}")]
    Conversion(String, String),
}
