use std::sync::Arc;
use std::time::Duration;
use esp32_nimble::{BLEAdvertisementData, BLECharacteristic, BLEDevice, NimbleProperties, uuid128};
use esp32_nimble::utilities::BleUuid;
use esp32_nimble::utilities::mutex::Mutex;
use espcam::espcam::{Camera, FrameBuffer};
use log::info;


// From https://www.uuidgenerator.net/version4
const IMAGE_SERVICE_UUID: BleUuid = uuid128!("35e7a030-2cf5-45ff-89a5-851d06e29d1b");
const RESPONSE_SERVICE_UUID: BleUuid = uuid128!("78b92928-f562-49db-827c-e9d83659f4cf");

const IMAGE_CHARACTERISTIC_UUID: BleUuid = uuid128!("dfead557-6578-495d-85f2-74b8db8010f5");
const RESPONSE_CHARACTERISTICS_UUID: BleUuid = uuid128!("fe4eef47-ce72-49bc-be82-24f77e9037fd");

pub fn ble_camera_main(camera: Camera) -> ! {

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
    let image_chunk_characteristic = image_sending_service.lock().create_characteristic(
        IMAGE_CHARACTERISTIC_UUID.clone(),
        NimbleProperties::READ | NimbleProperties::NOTIFY,
    );

    image_chunk_characteristic.lock().set_value(&vec![0x00; 765]);

    ble_advertiser
        .lock()
        .set_data(&mut BLEAdvertisementData::new()
            .name("Tommy's Terrific Torpedoes™")
            .add_service_uuid(RESPONSE_SERVICE_UUID.clone())
                  // .add_service_uuid(IMAGE_SERVICE_UUID.clone())
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
            .update_conn_params(clntdesc.conn_handle(), 8, 10, 0, 600)
            .unwrap();

        *is_connected_clone.lock() = true;
    });

    let is_connected_clone = is_connected.clone();

    // Define server disconnect behaviour
    server.on_disconnect(move |desc, reason| {
        println!("Disconnected, back to advertising\n    desc: {desc:?}\n    reason: {reason:?}");
        *is_connected_clone.lock() = false;
    });

    // let mut is_ready = Arc::new(Mutex::new(true));
    // let is_ready_clone = is_ready.clone();
    //
    // response_characteristic.lock().on_write(move |args| {
    //     info!("Notified: {:?}", args.desc());
    //     *is_ready_clone.lock() = true;
    // });

    loop {
        while !*is_connected.lock() {
            info!("Searching...");
            spin_sleep::sleep(Duration::from_secs(3));
        }

        while *is_connected.lock() {
            // info!("Send image!");
            send_image(&image_chunk_characteristic, &response_characteristic, &is_connected, camera.get_framebuffer().unwrap());
        }
    }
}

fn send_image(
    image_chunk_characteristic: &Arc<Mutex<BLECharacteristic>>,
    response_characteristic: &Arc<Mutex<BLECharacteristic>>,
    is_connected: &Arc<Mutex<bool>>,
    framebuffer: FrameBuffer
) {
    let pixel_len = 2;
    let scale = 2;
    let size = 765;

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

    // info!("bytes: {}", bytes.len());

    let chunk_received = Arc::new(Mutex::new(false));
    let chunk_received_clone = chunk_received.clone();

    response_characteristic.lock().on_write(move |args| {
        // info!("Chunk ready to be given!!: {:?}", args.desc());
        *chunk_received_clone.lock() = true;
    });

    let num_chunks = (bytes.len() - 3).div_ceil(size-1);

    for i in 0..num_chunks {
        while !*chunk_received.lock() && *is_connected.lock() {
            spin_sleep::sleep(Duration::from_millis(2));
        }

        if !*is_connected.lock() {
            break;
        }

        bytes.insert(i*size, i as u8);

        if i == 0 {
            bytes.insert(1, (size & 0xff) as u8);
            bytes.insert(2, (size >> 8) as u8);
            bytes.insert(3, num_chunks as u8);
        }

        let chunk = &bytes[i*size..((i+1)*size).min(bytes.len())];

        // info!("Sending chunk {} of {}: {:?}", i, num_chunks, &chunk[..7]);

        image_chunk_characteristic.lock()
            .set_value(&bytes[i*size..((i+1)*size).min(bytes.len())])
            .notify();

        *chunk_received.lock() = false;
    }
}