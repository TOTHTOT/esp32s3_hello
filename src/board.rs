use anyhow::{anyhow, Result};
use embedded_svc::wifi::Configuration;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspNvsPartition,
    wifi::{AuthMethod, ClientConfiguration, EspWifi},
};
use ws2812_esp32_rmt_driver::lib_smart_leds::Ws2812Esp32Rmt;

use esp_idf_svc::nvs::NvsDefault;
const WIFI_SSID: &str = "ERED";
const WIFI_PASSWD: &str = "www.ered.com";
pub struct BspEsp32S3CoreBoard<'d> {
    pub ws2812: Ws2812Esp32Rmt<'d>,
    pub wifi: EspWifi<'d>,
}

impl<'d> BspEsp32S3CoreBoard<'d> {
    pub fn new(peripherals: Peripherals) -> Result<Self> {
        let sysloop = EspSystemEventLoop::take()?;
        let nvs = EspNvsPartition::<NvsDefault>::take()?;
        let mut wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs))?;
        log::info!("wifi start");
        wifi.start()?;
        // 构造wifi名字和密码
        let mut ssid = heapless::String::<32>::new();
        let _ = ssid.push_str(WIFI_SSID);
        let mut password = heapless::String::<64>::new();
        let _ = password.push_str(WIFI_PASSWD);

        match wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid,
            password,
            auth_method: AuthMethod::WPA2Personal,
            ..Default::default()
        })) {
            Ok(_) => {
                log::info!("wifi start connect");
                if let Err(e) = wifi.connect() {
                    log::error!("wifi connect fail: {:?}", e);
                }
            }
            Err(e) => {
                log::warn!("wifi configuration fail: {:?}", e);
            }
        }

        let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio48)
            .map_err(|e| anyhow!("Ws2812Esp32Rmt error: {:?}", e))?;

        Ok(Self { ws2812, wifi })
    }
}
