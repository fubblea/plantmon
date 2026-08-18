pub(crate) mod dht;
pub(crate) mod error;
pub(crate) mod ldr;
pub(crate) mod soil;

use crate::alloc::string::ToString;
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

impl TryFrom<ComponentValue> for F32 {
    type Error = ComponentError;

    fn try_from(value: ComponentValue) -> Result<Self, Self::Error> {
        match value {
            ComponentValue::F32(value) => Ok(value),
            ComponentValue::F32Tuple(_, _) => Err(ComponentError::Conversion(
                "f32".to_string(),
                "(f32, f32)".to_string(),
            )),
        }
    }
}

impl TryFrom<ComponentValue> for (F32, F32) {
    type Error = ComponentError;

    fn try_from(value: ComponentValue) -> Result<Self, Self::Error> {
        match value {
            ComponentValue::F32(_) => Err(ComponentError::Conversion(
                "f32".to_string(),
                "(f32, f32)".to_string(),
            )),
            ComponentValue::F32Tuple(value1, value2) => Ok((value1, value2)),
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
            Err(e) => Err(ComponentError::Read(format!("{:?}", e))),
        }
    }
}
