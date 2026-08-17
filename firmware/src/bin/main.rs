#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::analog::adc::{Adc, AdcConfig};
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;

use micromath::F32;

use crate::components::dht::Dht;
use crate::components::ldr::Ldr;
use crate::components::soil::SoilMoisture;
use crate::components::{Adc1Component, GpioComponent};

extern crate alloc;

mod components;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let delay = Delay::new();

    // ---------------------------------------------------------
    // SENSOR 1 & 2: LDR and Soil Moisture on ADC
    // ---------------------------------------------------------
    let mut adc1_config = AdcConfig::new();
    let mut ldr = Ldr::from_pin_and_adc1(peripherals.GPIO5, &mut adc1_config);
    let mut moisture = SoilMoisture::from_pin_and_adc1(peripherals.GPIO6, &mut adc1_config);
    let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);

    // ---------------------------------------------------------
    // SENSOR 3: DHT22 on GPIO 18
    // ---------------------------------------------------------
    let mut dht_delay = Delay::new();
    let mut dht = Dht::from_pin_and_delay(peripherals.GPIO18, &mut dht_delay);

    // TODO: Spawn some tasks
    let _ = spawner;

    // Wait to let the sensors start
    delay.delay_millis(2000);

    println!("Sensors Configured! Starting main loop...\r");

    loop {
        let ldr_lux: F32 = match ldr.read(&mut adc1) {
            Ok(value) => value.into(),
            Err(e) => {
                match e {
                    components::error::ComponentError::ReadError(msg) => {
                        println!("LRD RearError, fallback to -1.0: {}", msg);
                    }
                    components::error::ComponentError::AdcConversionError(msg) => {
                        println!("LRD AdcConversionError, fallback to -1.0: {}", msg);
                    }
                }
                F32::from(-1.0) // Should never be negative, so this is a clear error value.
            }
        };
        let moisture_per: F32 = match moisture.read(&mut adc1) {
            Ok(value) => value.into(),
            Err(e) => {
                match e {
                    components::error::ComponentError::ReadError(msg) => {
                        println!("SoilMoisture ReadError, fallback to -1.0: {}", msg);
                    }
                    components::error::ComponentError::AdcConversionError(msg) => {
                        println!("SoilMoisture AdcConversionError, fallback to -1.0: {}", msg);
                    }
                }
                F32::from(-1.0) // Should never be negative, so this is a clear error value.
            }
        };
        let (temp_c, humidity_per) = match dht.read() {
            Ok(value) => value.into(),
            Err(e) => {
                match e {
                    components::error::ComponentError::ReadError(msg) => {
                        println!("SoilMoisture ReadError, fallback to -1.0: {}", msg);
                    }
                    components::error::ComponentError::AdcConversionError(msg) => {
                        println!("SoilMoisture AdcConversionError, fallback to -1.0: {}", msg);
                    }
                }
                (F32::from(-1.0), F32::from(-1.0)) // Should never be negative, so this is a clear error value.
            }
        };

        println!("-----------------------------\r");
        println!("Light Level {:.1}lux\r", ldr_lux);
        println!("Soil Moisture {:.1}%\r", moisture_per);
        println!("Temperature: {:.1}°C\r", temp_c);
        println!("Humidity: {:.1}%\r", humidity_per);

        delay.delay_millis(2000); // DHT22 minimum period
    }
}
