mod BLDC;
mod bluetooth_system;
mod terminal_system;
mod wifi_system;

use std::sync::Arc;
use esp_idf_svc::hal::prelude::*;
use espcam::espcam::Camera;
use log::info;
use BLDC::*;
use crate::bluetooth_system::ble_camera_main;
use crate::terminal_system::terminal_printer_main;
use crate::wifi_system::wifi_camera_main;


fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Hello, world!");

    let peripherals = Peripherals::take().unwrap();

    //esc_main(peripherals);

    info!("Starting camera");

    let camera = Camera::new(
        peripherals.pins.gpio32,
        peripherals.pins.gpio0,
        peripherals.pins.gpio5,
        peripherals.pins.gpio18,
        peripherals.pins.gpio19,
        peripherals.pins.gpio21,
        peripherals.pins.gpio36,
        peripherals.pins.gpio39,
        peripherals.pins.gpio34,
        peripherals.pins.gpio35,
        peripherals.pins.gpio25,
        peripherals.pins.gpio23,
        peripherals.pins.gpio22,
        peripherals.pins.gpio26,
        peripherals.pins.gpio27,
        esp_idf_sys::camera::pixformat_t_PIXFORMAT_RGB565,
        esp_idf_sys::camera::framesize_t_FRAMESIZE_240X240,
    )?;

    wifi_camera_main(camera, peripherals.modem);
    // ble_camera_main(camera);
    //terminal_printer_main(camera);
}