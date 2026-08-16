#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::analog::adc::{Adc, AdcConfig, Attenuation};
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Flex, InputConfig, Pull};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;

use embedded_dht_rs::dht22::Dht22;

mod components;

extern crate alloc;

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
    let mut ldr_pin = adc1_config.enable_pin(peripherals.GPIO5, Attenuation::_11dB);
    let mut moisture_pin = adc1_config.enable_pin(peripherals.GPIO6, Attenuation::_11dB);
    let mut adc1 = Adc::new(peripherals.ADC1, adc1_config);

    // ---------------------------------------------------------
    // SENSOR 3: DHT22 on GPIO 18
    // ---------------------------------------------------------
    let mut dht_pin = Flex::new(peripherals.GPIO18);
    let dht_config = InputConfig::default().with_pull(Pull::None);
    dht_pin.set_input_enable(true);
    dht_pin.set_output_enable(true);
    dht_pin.apply_input_config(&dht_config);
    dht_pin.set_high();

    let mut dht22 = Dht22::new(dht_pin, delay);

    // TODO: Spawn some tasks
    let _ = spawner;

    // Wait to let the sensors start
    delay.delay_millis(2000);

    println!("Sensors Configured! Starting main loop...\r");

    loop {
        let ldr_raw: u16 = nb::block!(adc1.read_oneshot(&mut ldr_pin)).unwrap_or(0);
        let moisture_raw: u16 = nb::block!(adc1.read_oneshot(&mut moisture_pin)).unwrap_or(0);

        let mut temp_c = 0.0;
        let mut humidity = 0.0;

        match dht22.read() {
            Ok(reading) => {
                temp_c = reading.temperature;
                humidity = reading.humidity;
            }
            Err(e) => println!("Failed to read DHT22: {:?}\r", e),
        }

        // Get current timestamp

        println!("-----------------------------\r");
        println!("Light Level (Raw): {}\r", ldr_raw);
        println!("Soil Moisture (Raw): {}\r", moisture_raw);
        println!("Temperature: {:.1}°C\r", temp_c);
        println!("Humidity: {:.1}%\r", humidity);

        delay.delay_millis(500);
    }
}
