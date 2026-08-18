use esp_hal::{
    analog::adc::{AdcConfig, AdcPin, Attenuation},
    peripherals::{ADC1, GPIO5},
};
use micromath::F32;

use crate::sensors::{Adc1Component, ComponentValue, error::ComponentError};

const ATTENUATION: Attenuation = Attenuation::_11dB;

pub(crate) struct Ldr<'a, Pin: esp_hal::gpio::Pin> {
    pin: AdcPin<Pin, ADC1<'a>>,
}

impl<'a> Adc1Component<'a, GPIO5<'a>> for Ldr<'a, GPIO5<'a>> {
    fn from_pin_and_adc1(pin: GPIO5<'a>, adc1_config: &mut AdcConfig<ADC1<'a>>) -> Self {
        let pin = adc1_config.enable_pin(pin, ATTENUATION);
        Self { pin }
    }

    fn get_pin(&mut self) -> &mut AdcPin<GPIO5<'a>, ADC1<'a>> {
        &mut self.pin
    }

    fn value_from_raw(&self, raw: u16) -> Result<ComponentValue, ComponentError> {
        // TODO: Do this properly
        Ok(ComponentValue::F32(F32::from(raw as f32)))
    }
}
