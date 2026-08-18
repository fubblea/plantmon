use esp_hal::{
    analog::adc::{AdcConfig, AdcPin, Attenuation},
    peripherals::{ADC1, GPIO6},
};
use micromath::F32;

use crate::sensors::{Adc1Component, ComponentValue, error::ComponentError};

const ATTENUATION: Attenuation = Attenuation::_11dB;

pub struct SoilMoisture<'a, Pin: esp_hal::gpio::Pin> {
    pin: AdcPin<Pin, ADC1<'a>>,
}

impl<'a> Adc1Component<'a, GPIO6<'a>> for SoilMoisture<'a, GPIO6<'a>> {
    fn from_pin_and_adc1(pin: GPIO6<'a>, adc1_config: &mut AdcConfig<ADC1<'a>>) -> Self {
        let pin = adc1_config.enable_pin(pin, ATTENUATION);
        Self { pin }
    }

    fn get_pin(&mut self) -> &mut AdcPin<GPIO6<'a>, ADC1<'a>> {
        &mut self.pin
    }

    fn value_from_raw(&self, raw: u16) -> Result<ComponentValue, ComponentError> {
        // TODO: Do this properly
        Ok(ComponentValue::F32(F32::from(raw as f32)))
    }
}
