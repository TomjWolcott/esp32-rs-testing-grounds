use std::str::FromStr;
use std::time::Duration;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::modem::Modem;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AccessPointConfiguration, ClientConfiguration, Configuration, EspWifi};
use espcam::espcam::Camera;
use heapless::String as HeaplessString;
use log::info;

#[derive(Debug)]
#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_name: &'static str,
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_password: &'static str,
}

pub fn wifi_camera_main(camera: Camera, modem: Modem) -> ! {
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi_driver = EspWifi::new(
        modem,
        sys_loop,
        Some(nvs)
    ).unwrap();

    // Configuration::AccessPoint(AccessPointConfiguration {
    //     ssid: Default::default(),
    //     ssid_hidden: false,
    //     channel: 0,
    //     secondary_channel: None,
    //     protocols: Default::default(),
    //     auth_method: Default::default(),
    //     password: Default::default(),
    //     max_connections: 0,
    // });

    wifi_driver.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: HeaplessString::<32>::from_str(CONFIG.wifi_ssid).unwrap(),
        password: HeaplessString::<64>::from_str(CONFIG.wifi_password).unwrap(),
        ..Default::default()
    })).unwrap();

    info!("Starting wifi... for {}, {}, {}", CONFIG.wifi_name, CONFIG.wifi_ssid, CONFIG.wifi_password);

    wifi_driver.start().unwrap();
    wifi_driver.connect().unwrap();

    while !wifi_driver.is_connected().unwrap(){
        let config = wifi_driver.get_configuration().unwrap();
        println!("Waiting for station {:?}", config);
        spin_sleep::sleep(Duration::from_secs(1));
    }

    println!("Should be connected now");
    loop{
        println!("IP info: {:?}", wifi_driver.sta_netif().get_ip_info().unwrap());
        spin_sleep::sleep(Duration::new(10,0));
    }
}