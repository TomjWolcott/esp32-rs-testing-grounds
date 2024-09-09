use colored::Colorize;
use espcam::espcam::Camera;
use log::info;

pub fn terminal_printer_main(camera: Camera) -> ! {
    let mut offset = 0;
    let pixel_len = 2;
    let scale = 3;
    // let wid = 50;
    // let hi = 50;

    info!("Starting loop");
    loop {
        if let Some(framebuffer) = camera.get_framebuffer() {
            let (width, height) = (framebuffer.width(), framebuffer.height());
            let mut string = "\n".to_string();

            info!("#pixels: {} vs {}, width: {}, height: {}", framebuffer.data().len() / pixel_len, width * height, width, height);
            for i in 0..width/scale {
                for j in 0..height/scale {
                    let index = (i*scale * width + j*scale) * pixel_len + offset;
                    let bits = ((framebuffer.data()[index] as u32) << 8) | framebuffer.data()[index+1] as u32;
                    let (r, g, b) = ((bits >> 11) as u8, (0b111111 & (bits >> 5)) as u8, (0b11111 & bits) as u8);
                    // let r = framebuffer.data()[index];
                    let pixel = "██".truecolor(r<<3, g<<2, b<<3);
                    string.push_str(pixel.to_string().as_str());
                }
                string.push('\n');
            }

            println!("{string}");
        }
    }
}