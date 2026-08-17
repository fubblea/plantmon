pub(crate) mod dht;
pub(crate) mod error;
pub(crate) mod ldr;
pub(crate) mod soil;

use alloc::format;
use esp_hal::{
    Blocking,
    analog::adc::{Adc, AdcConfig, AdcPin},
    delay::Delay,
    peripherals::ADC1,
};
use micromath::F32;

use crate::components::error::ComponentError;

pub(crate) enum ComponentValue {
    F32(F32),
    F32Tuple(F32, F32),
}

impl From<ComponentValue> for F32 {
    fn from(val: ComponentValue) -> Self {
        match val {
            ComponentValue::F32(value) => value,
            ComponentValue::F32Tuple(_, _) => {
                panic!("Cannot convert F32Tuple to F32 directly.")
            }
        }
    }
}

impl From<ComponentValue> for (F32, F32) {
    fn from(val: ComponentValue) -> Self {
        match val {
            ComponentValue::F32(_) => {
                panic!("Cannot convert F32 to (F32, F32) directly.")
            }
            ComponentValue::F32Tuple(value1, value2) => (value1, value2),
        }
    }
}

pub(crate) trait GpioComponent<'a, Pin: esp_hal::gpio::Pin> {
    fn from_pin_and_delay(pin: Pin, delay: &'a mut Delay) -> Self;
    fn read(&mut self) -> Result<ComponentValue, error::ComponentError>;
}

pub(crate) trait Adc1Component<'a, Pin: esp_hal::gpio::Pin + esp_hal::analog::adc::AdcChannel> {
    fn from_pin_and_adc1(pin: Pin, adc1_config: &mut AdcConfig<ADC1<'a>>) -> Self;
    fn get_pin(&mut self) -> &mut AdcPin<Pin, ADC1<'a>>;
    fn value_from_raw(&self, raw: u16) -> Result<ComponentValue, ComponentError>;
    fn read(
        &mut self,
        adc1: &mut Adc<'a, ADC1<'a>, Blocking>,
    ) -> Result<ComponentValue, error::ComponentError> {
        match nb::block!(adc1.read_oneshot(self.get_pin())) {
            Ok(value) => Ok(self.value_from_raw(value)?),
            Err(e) => Err(ComponentError::ReadError(format!("{:?}", e))),
        }
    }
}
