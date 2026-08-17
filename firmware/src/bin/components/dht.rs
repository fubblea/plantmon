use alloc::format;
use embedded_dht_rs::dht22::Dht22;
use esp_hal::gpio::{DriveMode, Flex, InputConfig, OutputConfig, Pull};
use esp_hal::{delay::Delay, peripherals::GPIO18};

use crate::components::error::ComponentError;
use crate::components::{ComponentValue, GpioComponent};

pub(crate) struct Dht<'a> {
    dht22: Dht22<Flex<'a>, &'a mut Delay>,
}

impl<'a> GpioComponent<'a, GPIO18<'a>> for Dht<'a> {
    fn from_pin_and_delay(pin: GPIO18<'a>, delay: &'a mut Delay) -> Self {
        let mut dht_pin = Flex::new(pin);
        dht_pin.apply_output_config(
            &OutputConfig::default()
                .with_drive_mode(DriveMode::OpenDrain)
                .with_pull(Pull::Up),
        );
        dht_pin.apply_input_config(&InputConfig::default().with_pull(Pull::Up));
        dht_pin.set_input_enable(true);
        dht_pin.set_output_enable(true);
        dht_pin.set_high();

        let dht22 = Dht22::new(dht_pin, delay);
        Self { dht22 }
    }

    fn read(&mut self) -> Result<ComponentValue, ComponentError> {
        match self.dht22.read() {
            Ok(reading) => Ok(ComponentValue::F32Tuple(
                reading.temperature.into(), // Should be in Celsius
                reading.humidity.into(),    // Should be in percentage
            )),
            Err(e) => Err(ComponentError::ReadError(format!("{:?}", e))),
        }
    }
}
