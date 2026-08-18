#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types."
)]
#![deny(clippy::large_stack_frames)]

use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use esp_backtrace as _;
use esp_hal::analog::adc::{Adc, AdcConfig};
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::rng::Rng;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_radio::wifi::sta::StationConfig;
use static_cell::StaticCell;

use micromath::F32;

mod sensors;

use embassy_net::{Config as NetConfig, Runner, StackResources};
use esp_radio::wifi::{Config as WifiConfig, ControllerConfig, Interface};
use sensors::dht::Dht;
use sensors::ldr::Ldr;
use sensors::soil::SoilMoisture;
use sensors::tasks::{adc_sensor_task, dht_sensor_task};
use sensors::{Adc1Component, GpioComponent};

extern crate alloc;

const CONNECTIONS_MAX: usize = 1;
#[allow(unused)]
const L2CAP_CHANNELS_MAX: usize = 1;

// Network static resources
static STACK_RESOURCES: StaticCell<embassy_net::StackResources<CONNECTIONS_MAX>> =
    StaticCell::new();

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

pub(crate) enum SensorData {
    Adc { light_lux: F32, moisture_per: F32 },
    Dht { temp_c: F32, humidity_per: F32 },
}

// Capacity of 4 messages
static SENSOR_CHANNEL: Channel<CriticalSectionRawMutex, SensorData, 4> = Channel::new();

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await;
}

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    rtt_target::rtt_init_defmt!();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // The following pins are used to bootstrap the chip. They are available
    // for use, but check the datasheet of the module for more information on them.
    // - GPIO4
    // - GPIO5
    // - GPIO8
    // - GPIO9
    // - GPIO15
    // These GPIO pins are in use by some feature of the module and should not be used.
    let _ = peripherals.GPIO24;
    let _ = peripherals.GPIO25;
    let _ = peripherals.GPIO26;
    let _ = peripherals.GPIO27;
    let _ = peripherals.GPIO28;
    let _ = peripherals.GPIO29;
    let _ = peripherals.GPIO30;

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    println!("Embassy initialized!\r");

    // Configure SSID and Password
    let station_config = WifiConfig::Station(
        StationConfig::default()
            .with_ssid(env!("WIFI_SSID"))
            .with_password(env!("WIFI_PASSWORD").into()),
    );

    // We only need WiFi
    let (mut wifi_controller, interfaces) = esp_radio::wifi::new(
        peripherals.WIFI,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller");

    let (_stack, runner) = embassy_net::new(
        interfaces.station,
        NetConfig::dhcpv4(Default::default()),
        STACK_RESOURCES.init(StackResources::<CONNECTIONS_MAX>::new()),
        Rng::new().random() as u64,
    );

    println!("Connecting to Wi-Fi...\r");
    wifi_controller.connect_async().await.unwrap();
    println!("Wi-Fi Connected!\r");

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

    println!("Sensors configured!\r");

    spawner.spawn(adc_sensor_task(ldr, moisture, adc1).expect("Unable to start ADC task"));
    spawner.spawn(dht_sensor_task(dht).expect("Unable to start DHT task"));
    spawner.spawn(net_task(runner).expect("Unable to spawn network runner"));

    println!("Tasks spawned! Starting main loop...\r");

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
