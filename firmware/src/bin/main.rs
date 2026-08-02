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
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Level, Output, OutputConfig};
use esp_hal::timer::timg::TimerGroup;
use esp_println::println;
use esp_radio::ble::controller::BleConnector;
use panic_rtt_target as _;
use trouble_host::prelude::*;

extern crate alloc;

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
    // generator version: 1.3.0
    // generator parameters: --chip esp32c6 -o esp32c6-wroom-1 -o unstable-hal -o alloc -o wifi -o embassy -o ble-trouble -o probe-rs -o defmt -o panic-rtt-target -o embedded-test -o wokwi -o vscode -o stable-x86_64-pc-windows-msvc

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

    // Setup LED on GPIO8
    let mut led = Output::new(peripherals.GPIO8, Level::High, OutputConfig::default());
    let led_delay = Delay::new();

    // TODO: Spawn some tasks
    let _ = spawner;

    println!("init::Starting main loop!\r");

    loop {
        println!("LED low\r");
        led.set_low();
        led_delay.delay_millis(1000);
        println!("LED high\r");
        led.set_high();
        led_delay.delay_millis(1000);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.1.0/examples
}
