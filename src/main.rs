mod BLDC;
mod frame_handling;

use std::sync::Arc;
use BLDC::*;

use std::time::Duration;
use colored::Colorize;
use esp32_nimble::{BLEAdvertisementData, BLECharacteristic, BLEClient, BLEDevice, NimbleProperties, uuid128};
use esp32_nimble::enums::{ConnMode, DiscMode};
use esp32_nimble::utilities::BleUuid;
use esp32_nimble::utilities::mutex::Mutex;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{IOPin, Level, OutputPin, PinDriver, Pins};
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::rmt::{FixedLengthSignal, PinState, Pulse, PulseTicks, TxRmtDriver};
use esp_idf_svc::hal::rmt::config::TransmitConfig;
use esp_idf_svc::hal::task::notify;
use espcam::ble::ble_advertise_task;
use espcam::espcam::{Camera, FrameBuffer};
use espcam::wifi_handler::my_wifi;
use log::{info, log};
use rgb::RGB8;

// From https://www.uuidgenerator.net/version4
const IMAGE_SERVICE_UUID: BleUuid = uuid128!("35e7a030-2cf5-45ff-89a5-851d06e29d1b");
const RESPONSE_SERVICE_UUID: BleUuid = uuid128!("78b92928-f562-49db-827c-e9d83659f4cf");

const IMAGE_CHARACTERISTICS_UUID: BleUuid = uuid128!("dfead557-6578-495d-85f2-74b8db8010f5");
const RESPONSE_CHARACTERISTICS_UUID: BleUuid = uuid128!("fe4eef47-ce72-49bc-be82-24f77e9037fd");

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Hello, world!");

    // unsafe {
    //     // info!("PSRAM size: {}", esp_psram_get_size());
    //     info!("Is PSRAM inititialized? {}", esp_psram_is_initialized());
    // }

    let peripherals = Peripherals::take().unwrap();

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

    let ble_device = BLEDevice::take();
    let ble_advertiser = ble_device.get_advertising();
    let server = ble_device.get_server();

    // let mut ad_data = BLEAdvertisementData::new();

    // ble_advertiser.lock()
    //     .advertisement_type(ConnMode::Und)
    //     .disc_mode(DiscMode::Gen)
    //     .scan_response(true);

    let response_service = server.create_service(RESPONSE_SERVICE_UUID.clone());

    let response_characteristic = response_service.lock().create_characteristic(
        RESPONSE_CHARACTERISTICS_UUID.clone(),
        NimbleProperties::READ | NimbleProperties::WRITE | NimbleProperties::NOTIFY,
    );

    response_characteristic.lock().set_value(&[0x00; 50]);

    // Create a service with custom UUID
    let image_sending_service = server.create_service(IMAGE_SERVICE_UUID.clone());

    // Create a characteristic to associate with created service
    let sending_characteristics: Vec<_> = (0..40).map(|_| {
        let characteristic = image_sending_service.lock().create_characteristic(
            IMAGE_CHARACTERISTICS_UUID.clone(),
            NimbleProperties::READ | NimbleProperties::NOTIFY,
        );

        characteristic.lock().set_value(&vec![0x00; 512]);

        characteristic
    }).collect();

    ble_advertiser
        .lock()
        .set_data(&mut BLEAdvertisementData::new()
            .name("Tommy's Terrific Torpedoes™")
            .add_service_uuid(RESPONSE_SERVICE_UUID.clone())
        ).unwrap();

    // info!("ble_advertiser: {:?}", ble_advertiser);

    ble_advertiser.lock().start().unwrap();

    let is_connected = Arc::new(Mutex::new(false));
    let is_connected_clone = is_connected.clone();

    server.on_connect(move |server, clntdesc| {
        // Print connected client data
        info!("We're connected!! {:?}", clntdesc);

        // Update connection parameters
        server
            .update_conn_params(clntdesc.conn_handle(), 24, 48, 0, 600)
            .unwrap();

        *is_connected_clone.lock() = true;
    });

    let is_connected_clone = is_connected.clone();

    // Define server disconnect behaviour
    server.on_disconnect(move |desc, reason| {
        println!("Disconnected, back to advertising\n    desc: {desc:?}\n    reason: {reason:?}");
        *is_connected_clone.lock() = false;
    });

    let mut is_ready = Arc::new(Mutex::new(false));
    let is_ready_clone = is_ready.clone();

    response_characteristic.lock().on_notify_tx(move |notify_tx| {
        info!("Notified: {:?}", notify_tx.status());
        *is_ready_clone.lock() = true;
    });

    loop {
        while !*is_connected.lock() {
            info!("Searching...");
            spin_sleep::sleep(Duration::from_secs(3));
        }

        while !*is_ready.lock() && *is_connected.lock() {
            info!("Waiting for response...");
            spin_sleep::sleep(Duration::from_secs_f32(0.1));
        }

        if *is_connected.lock() {
            send_image(&sending_characteristics, camera.get_framebuffer().unwrap());
            *is_ready.lock() = false;
        }
    }

    // let mut bldc_driver = BldcDriver::new(
    //     (peripherals.pins.gpio13, peripherals.pins.gpio12),
    //     (peripherals.pins.gpio14, peripherals.pins.gpio15),
    //     (peripherals.pins.gpio16, peripherals.pins.gpio2),
    // )?;

    // let mut offset = 0;
    // let pixel_len = 2;
    // let scale = 3;
    // // let wid = 50;
    // // let hi = 50;

    // info!("Starting loop");
    // loop {
    //     if let Some(framebuffer) = camera.get_framebuffer() {
    //         let (width, height) = (framebuffer.width(), framebuffer.height());
    //         let mut string = "\n".to_string();
    //
    //         info!("#pixels: {} vs {}, width: {}, height: {}", framebuffer.data().len() / pixel_len, width * height, width, height);
    //         for i in 0..width/scale {
    //             for j in 0..height/scale {
    //                 let index = (i*scale * width + j*scale) * pixel_len + offset;
    //                 let bits = ((framebuffer.data()[index] as u32) << 8) | framebuffer.data()[index+1] as u32;
    //                 let (r, g, b) = ((bits >> 11) as u8, (0b111111 & (bits >> 5)) as u8, (0b11111 & bits) as u8);
    //                 // let r = framebuffer.data()[index];
    //                 let pixel = "██".truecolor(r<<3, g<<2, b<<3);
    //                 string.push_str(pixel.to_string().as_str());
    //             }
    //             string.push('\n');
    //         }
    //
    //         println!("{string}");
    //     }
    //
    //     // bldc_driver.send_sequence(
    //     //     Duration::from_secs_f32(1e0),
    //     //     Duration::from_secs_f32(9e0)
    //     // )?;
    // }

    // Ok(())
}

fn send_image(characteristics: &Vec<Arc<Mutex<BLECharacteristic>>>, framebuffer: FrameBuffer) {
    let pixel_len = 2;
    let scale = 3;
    let size = 512;

    let (width_b, height_b) = (
        ((framebuffer.width() / scale) as u32).to_le_bytes(),
        ((framebuffer.height() / scale) as u32).to_le_bytes()
    );

    let mut bytes = [width_b, height_b].concat();

    for i in 0..framebuffer.height() / scale {
        for j in 0..framebuffer.width() / scale {
            let index = (i*scale * framebuffer.width() + j*scale) * pixel_len;

            bytes.push(framebuffer.data()[index]);
            bytes.push(framebuffer.data()[index+1]);
        }
    }

    info!("bytes: {}, available bytes: {}", bytes.len(), characteristics.len() * (size - 1));

    for (i, characteristic) in characteristics.iter().enumerate() {
        if i*size > bytes.len() {
            characteristic.lock().set_value(&[]).notify();
        } else if (i+1)*size > bytes.len() {
            bytes.insert(i*size, i as u8);

            characteristic.lock()
                .set_value(&bytes[i*size..])
                .notify();
        } else {
            bytes.insert(i*size, i as u8);

            characteristic.lock()
                .set_value(&bytes[i*size..(i+1)*size])
                .notify();
        }
    }
}