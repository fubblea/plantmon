use crate::sensors::Adc1Component;
use crate::sensors::GpioComponent;
use crate::sensors::dht::Dht;
use core::marker::Sized;

use embassy_time::Duration;

use embassy_time::Timer;
use micromath::F32;

use crate::SensorData;

use crate::SENSOR_CHANNEL;
use core::convert::From;

use esp_hal::Blocking;

use esp_hal::analog::adc::Adc;

use crate::sensors::soil::SoilMoisture;

use crate::sensors::ldr::Ldr;

#[embassy_executor::task]
pub(crate) async fn adc_sensor_task(
    mut ldr: Ldr<'static, esp_hal::peripherals::GPIO5<'static>>,
    mut moisture: SoilMoisture<'static, esp_hal::peripherals::GPIO6<'static>>,
    mut adc1: Adc<'static, esp_hal::peripherals::ADC1<'static>, Blocking>,
) {
    loop {
        let light_lux: F32 = ldr
            .read(&mut adc1)
            .await
            .and_then(|raw| raw.try_into())
            .unwrap_or(F32::from(-1.0));
        let moisture_per: F32 = moisture
            .read(&mut adc1)
            .await
            .and_then(|raw| raw.try_into())
            .unwrap_or(F32::from(-1.0));

        // Send data to the main task
        let _ = SENSOR_CHANNEL
            .send(SensorData::Adc {
                light_lux,
                moisture_per,
            })
            .await;

        // Yield the CPU for 2 seconds
        Timer::after(Duration::from_secs(2)).await;
    }
}

#[embassy_executor::task]
pub(crate) async fn dht_sensor_task(mut dht: Dht<'static>) {
    loop {
        let (temp_c, humidity_per) = dht
            .read()
            .await
            .and_then(|raw| raw.try_into())
            .unwrap_or((F32::from(-1.0), F32::from(-1.0)));

        // Send data to the main task
        let _ = SENSOR_CHANNEL
            .send(SensorData::Dht {
                temp_c,
                humidity_per,
            })
            .await;

        // Yield the CPU for 2 seconds
        Timer::after(Duration::from_secs(2)).await;
    }
}
