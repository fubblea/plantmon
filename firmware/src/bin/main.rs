#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use bt_hci::controller::ExternalController;
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_hal::analog::adc;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::i2c::master::Config as I2cConfig; // for convenience, importing as alias
use esp_hal::i2c::master::I2c;
use esp_hal::time::Rate;
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_radio::ble::controller::BleConnector;

use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::Point,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{I2CDisplayInterface, Ssd1306Async, prelude::*};

use micromath::F32Ext;
use trouble_host::prelude::*;

extern crate alloc;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// BLE host resources configuration
const CONNECTIONS_MAX: usize = 1;
const L2CAP_CHANNELS_MAX: usize = 1;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[esp_rtos::main]
async fn main(spawner: Spawner) -> ! {
    rtt_target::rtt_init_defmt!();

    // Initialize the peripherals and the system clock
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

    println!("---------------------\r");

    // Creates heap memory regions. Reclaims memory from the booloader for heap.
    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 65536);
    // COEX needs more RAM - so we've added some more
    esp_alloc::heap_allocator!(size: 64 * 1024);

    // Start the RTOS scheduler. This will start the main task and never return.
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    println!("init::Embassy initialized!\r");

    // Initialize the Wi-Fi controller and interfaces
    let (mut _wifi_controller, _interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, Default::default())
            .expect("Failed to initialize Wi-Fi controller");
    // find more examples https://github.com/embassy-rs/trouble/tree/main/examples/esp32
    let transport = BleConnector::new(peripherals.BT, Default::default()).unwrap();
    let ble_controller = ExternalController::<_, 1>::new(transport);
    let mut resources: HostResources<DefaultPacketPool, CONNECTIONS_MAX, L2CAP_CHANNELS_MAX> =
        HostResources::new();
    let _stack = trouble_host::new(ble_controller, &mut resources);

    println!("init::WiFi and BLE initialized!\r");

    // Setup LED on GPIO8
    let mut led = Output::new(peripherals.GPIO8, Level::High, OutputConfig::default());
    let led_delay = Delay::new();

    println!("init::LED initialized!\r");

    // Setup temp sensor on GPIO4
    let mut adc1_config = adc::AdcConfig::new();
    let mut adc1_pin = adc1_config.enable_pin(peripherals.GPIO2, adc::Attenuation::_11dB);
    let mut temp_sensor = adc::Adc::new(peripherals.ADC1, adc1_config);
    const TEMP_SENSOR_BETA: f32 = 3950.0; // Beta value for the thermistor

    println!("init::Temp sensor initialized!\r");

    // Setup OLED on I2C (GPI18=SDA, GPI19=SCL)
    let i2c_bus = I2c::new(
        peripherals.I2C0,
        I2cConfig::default().with_frequency(Rate::from_khz(400)),
    )
    .unwrap()
    .with_sda(peripherals.GPIO18)
    .with_scl(peripherals.GPIO19)
    .into_async();
    let interface = I2CDisplayInterface::new(i2c_bus);
    // initialize the display
    let mut display = Ssd1306Async::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().await.unwrap();

    println!("init::Display initialized!\r");

    // TODO: Spawn some tasks
    let _ = spawner;

    println!("init::Starting main loop!\r");

    loop {
        println!("Updating OLED display\r");
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        Text::with_baseline("Hello, Ajay!", Point::new(0, 16), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().await.unwrap();

        println!("LED low\r");
        led.set_low();

        let temp_raw = nb::block!(temp_sensor.read_oneshot(&mut adc1_pin)).unwrap();
        let temp_celsius = 1.0
            / ((1.0 / (1023.0 / temp_raw as f32 - 1.0)).log(10.0) / TEMP_SENSOR_BETA
                + 1.0 / 298.15)
            - 273.15;
        println!("Temperature: {:.2} °C\r", temp_celsius);

        led_delay.delay_millis(1000);

        println!("Updating OLED display\r");
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();
        Text::with_baseline("Hello, Rust!", Point::new(0, 16), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().await.unwrap();

        println!("LED high\r");
        led.set_high();
        led_delay.delay_millis(1000);
    }
}
