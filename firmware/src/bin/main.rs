#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types."
)]
// #![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use esp_backtrace as _;
use esp_hal::Blocking;
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

enum SensorData {
    Adc { light_lux: F32, moisture_per: F32 },
    Dht { temp_c: F32, humidity_per: F32 },
}

// Capacity of 4 messages
static SENSOR_CHANNEL: Channel<CriticalSectionRawMutex, SensorData, 4> = Channel::new();

#[embassy_executor::task]
async fn adc_sensor_task(
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
async fn dht_sensor_task(mut dht: Dht<'static>) {
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

    // ---------------------------------------------------------
    // SENSOR 1 & 2: LDR and Soil Moisture on ADC
    // ---------------------------------------------------------
    let mut adc1_config = AdcConfig::new();
    let ldr = Ldr::from_pin_and_adc1(peripherals.GPIO5, &mut adc1_config);
    let moisture = SoilMoisture::from_pin_and_adc1(peripherals.GPIO6, &mut adc1_config);
    let adc1 = Adc::new(peripherals.ADC1, adc1_config);

    // ---------------------------------------------------------
    // SENSOR 3: DHT22 on GPIO 18
    // ---------------------------------------------------------
    let dht_delay = Delay::new();
    let dht = Dht::from_pin_and_delay(peripherals.GPIO18, dht_delay);

    spawner.spawn(adc_sensor_task(ldr, moisture, adc1).expect("Unable to start ADC task"));
    spawner.spawn(dht_sensor_task(dht).expect("Unable to start DHT task"));

    println!("Sensors Configured! Starting main loop...\r");

    // Initial values
    let mut last_light_lux = F32::from(-1.0);
    let mut last_moisture_per = F32::from(-1.0);
    let mut last_temp_c = F32::from(-1.0);
    let mut last_humidity_per = F32::from(-1.0);

    loop {
        match SENSOR_CHANNEL.receive().await {
            SensorData::Adc {
                light_lux,
                moisture_per,
            } => {
                last_light_lux = light_lux;
                last_moisture_per = moisture_per;
            }
            SensorData::Dht {
                temp_c,
                humidity_per,
            } => {
                last_temp_c = temp_c;
                last_humidity_per = humidity_per;
            }
        }

        println!("-----------------------------\r");
        println!("Light Level {:.1}lux\r", last_light_lux);
        println!("Soil Moisture {:.1}%\r", last_moisture_per);
        println!("Temperature: {:.1}°C\r", last_temp_c);
        println!("Humidity: {:.1}%\r", last_humidity_per);
    }
}
