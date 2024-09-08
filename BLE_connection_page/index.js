const camStream = document.getElementById('cam_stream');
const connectButton = document.getElementById('connect_button');
const browseButton = document.getElementById('browse_button');
const debugText = document.getElementById('bt-debug');

browseButton.onclick = () => {
    navigator.bluetooth.requestDevice({
        acceptAllDevices: true,
        optionalServices: [] // Required to access service later.
    }).then(device => {
        console.log("Device: ", device);
        debugText.innerHTML = JSON.stringify(device, null, 2);
    }).catch(error => {
        console.error(error);
    });
}

connectButton.onclick = () => {
    navigator.bluetooth.requestDevice({
        filters: [{
            name: "Tommy's Terrific Torpedoes™"
        }, {
            name: "nimble"
        }, {
            namePrefix: "Tommy"
        }],
        optionalServices: ["35e7a030-2cf5-45ff-89a5-851d06e29d1b", "78b92928-f562-49db-827c-e9d83659f4cf"] // Required to access service later.
    }).then(device => device.gatt.connect())
        .then(handleConnection)
        .catch(error => {
            console.error(error);
        });
};

async function handleConnection(server) {
    console.log("CONNECTED!!", server);

    connectButton.innerHTML = "Reconnect";
    connectButton.onclick = () => {
        console.log("Time to reconnect!");
        server.device.gatt.connect()
            .then(handleConnection)
            .catch(error => {
                console.error("Reconnect", error);
            });
    }

    let response_characteristic = await server.getPrimaryService("78b92928-f562-49db-827c-e9d83659f4cf")
        .then(service => service.getCharacteristic("fe4eef47-ce72-49bc-be82-24f77e9037fd"))

    console.log("Get service");
    let image_characteristics = await server.getPrimaryService("35e7a030-2cf5-45ff-89a5-851d06e29d1b")
        .then(service => service.getCharacteristics("dfead557-6578-495d-85f2-74b8db8010f5"));

    while (true) {
        console.log("Read value", image_characteristics);
        await image_characteristics.stopNotifications();

        try {
            let arrays = await Promise.all(image_characteristics.map(c => c.readValue()));
            console.log("Arrays: ", arrays);
            let bytes = arrays
                .map(array => [...new Uint8Array(array.buffer)])
                .filter(bytes => bytes.length > 0);

            console.log("Bytes1: ", [...bytes]);
            // Sorts the messages into order
            bytes.sort((a, b) => a[0] - b[0]);
            console.log("Bytes2: ", [...bytes]);

            bytes = bytes.map(array => array.slice(1)).flat();

            console.log("Update image, ", bytes);
            updateImageSource(bytes);
        } catch (err) {
            if (err.message.includes("connect")) {
                console.log("Time to reconnect!");
                server.device.gatt.connect()
                    .then(handleConnection)
                    .catch(error => {
                        console.error("Reconnect", error);
                    });
                break;
            } else {
                console.error(err);
            }
        }

        await new Promise((resolve, _) => setTimeout(() => resolve(), 300));
        await response_characteristic.startNotification();
    }
}

function updateImageSource(bytes) {
    let base64 = btoa(String.fromCharCode(
        ...gridRgb565ToBmp888(bytes)
    ));

    camStream.setAttribute("src", `data:image/bmp;base64,${base64}`);
}


/*
* bytes (be): [
*   _, _, _, _, // width
*   _, _, _, _, // height
*   _, _, ... // Pixel data [RGB565, ...]
* ]
*
* */
function gridRgb565ToBmp888(bytes) {
    /*
    let mut offset = 0;
    let pixel_len = 2;
    let scale = 8;

    let (width_b, height_b) = (
        ((framebuffer.width() / scale) as u32).to_le_bytes(),
        ((framebuffer.height() / scale) as u32).to_le_bytes()
    );

    info!("{width_b:?} -- {height_b:?}");

    let mut bytes: Vec<u8> = vec![
        0x42, 0x4D, // BM
        0x00, 0x00, 0x00, 0x00, // File size
        0x00, 0x00, 0x00, 0x00, // Not important
        0x36, 0x00, 0x00, 0x00, // offset -- where the pixel array starts

        0x28, 0x00, 0x00, 0x00, // Num bytes in header
        width_b[0], width_b[1], width_b[2], width_b[3], // width (80)
        height_b[0], height_b[1], height_b[2], height_b[3], // height (80)
        0x01, 0x00, // 1 color plane
        0x18, 0x00, // # bits per pixel
        0x00, 0x00, 0x00, 0x00, // No compression
        0x10, 0x00, 0x00, 0x00, // Size of raw bitmap data
        0x13, 0x0B, 0x00, 0x00, // print res
        0x13, 0x0B, 0x00, 0x00, // print res
        0x00, 0x00, 0x00, 0x00, // # colors in palette
        0x00, 0x00, 0x00, 0x00, // # no important colors
    ];

    let (width, height) = (framebuffer.width(), framebuffer.height());

    info!("#pixels: {} vs {}, width: {}, height: {}", framebuffer.data().len() / pixel_len, width * height, width, height);
    for i in 0..height/scale {
        for j in 0..width/scale {
            let index = (i * scale * width + j * scale) * pixel_len + offset;
            let bits = ((framebuffer.data()[index] as u32) << 8) | framebuffer.data()[index + 1] as u32;
            let (r, g, b) = ((bits >> 11) as u8, (0b111111 & (bits >> 5)) as u8, (0b11111 & bits) as u8);

            bytes.push(r << 3);
            bytes.push(g << 2);
            bytes.push(b << 3);
        }

        for _ in 0..((1000000000 - width / scale) % 4) {
            bytes.push(0x00);
        }
    }

    info!("sending bytes, len: {}", bytes.len());

    let len = (bytes.len() as u32).to_be_bytes();
    bytes[2] = len[0];
    bytes[3] = len[1];
    bytes[4] = len[2];
    bytes[5] = len[3];
    */

    let width_b = bytes.slice(0, 4);
    let width = width_b[0] | (width_b[1] << 8) | (width_b[2] << 16) | (width_b[3] << 24);
    let height_b = bytes.slice(4, 8);
    let height = height_b[0] | (height_b[1] << 8) | (height_b[2] << 16) | (height_b[3] << 24);
    bytes = bytes.slice(8);
    let bmpBytes = [
        0x42, 0x4D, // BM
        0x00, 0x00, 0x00, 0x00, // File size
        0x00, 0x00, 0x00, 0x00, // Not important
        0x36, 0x00, 0x00, 0x00, // offset -- where the pixel array starts

        0x28, 0x00, 0x00, 0x00, // Num bytes in header
        width_b[0], width_b[1], width_b[2], width_b[3], // width
        height_b[0], height_b[1], height_b[2], height_b[3], // height
        0x01, 0x00, // 1 color plane
        0x18, 0x00, // # bits per pixel
        0x00, 0x00, 0x00, 0x00, // No compression
        0x10, 0x00, 0x00, 0x00, // Size of raw bitmap data
        0x13, 0x0B, 0x00, 0x00, // print res
        0x13, 0x0B, 0x00, 0x00, // print res
        0x00, 0x00, 0x00, 0x00, // # colors in palette
        0x00, 0x00, 0x00, 0x00, // # no important colors
    ];

    console.log("bmpBytes", bmpBytes);
    console.log("width", width);
    console.log("height", height);

    for (let i = 0; i < width; i += 2) {
        for (let j = 0; j < height; j++) {
            let index = (i * width + j) * 2;
            let bits = (bytes[index] << 8) | bytes[index + 1];
            let r = (bits >> 11) << 3;
            let g = ((bits >> 5) & 0b111111) << 2;
            let b = (bits & 0b11111) << 3;
            bmpBytes.push(r, g, b);
        }

        for (let j = 0; j < ((10000000 - width) % 4); j++) {
            bmpBytes.push(0x00);
        }
    }

    let len = bmpBytes.length;
    bmpBytes[2] = len & 0xFF;
    bmpBytes[3] = (len >> 8) & 0xFF;
    bmpBytes[4] = (len >> 16) & 0xFF;
    bmpBytes[5] = (len >> 24) & 0xFF;

    return bmpBytes
}