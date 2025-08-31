use anyhow::{anyhow, Result};
use embedded_svc::wifi::Configuration;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::{
    bt::*,
    eventloop::EspSystemEventLoop,
    nvs::EspNvsPartition,
    wifi::{AuthMethod, ClientConfiguration, EspWifi},
};
use ws2812_esp32_rmt_driver::lib_smart_leds::Ws2812Esp32Rmt;

use esp_idf_svc::nvs::NvsDefault;
// 默认连接的wifi
const WIFI_SSID: &str = "esp32_2.4G";
const WIFI_PASSWD: &str = "12345678..";
pub struct BspEsp32S3CoreBoard<'d> {
    pub ws2812: Ws2812Esp32Rmt<'d>,
    pub wifi: Option<EspWifi<'d>>,
    pub ble: Option<BtDriver<'d, Ble>>,
    wifi_ssid: String,
    wifi_password: String,
}

#[allow(dead_code)]
impl<'d> BspEsp32S3CoreBoard<'d> {
    pub fn new(peripherals: Peripherals) -> Result<Self> {
        let sysloop = EspSystemEventLoop::take()?;
        let nvs = EspNvsPartition::<NvsDefault>::take()?;
        let mut wifi = None;
        let mut ble = None;

        // 目前所知, wifi和ble可能不能同时使用因为 modem 不支持 copy 和 Arc 特性, 只能被一个设备持有
        if false {
            wifi = Some(EspWifi::new(peripherals.modem, sysloop, Some(nvs.clone()))?);
        } else {
            ble = Some(BtDriver::<Ble>::new(peripherals.modem, Some(nvs.clone()))?);
        }
        let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio48)
            .map_err(|e| anyhow!("Ws2812Esp32Rmt error: {:?}", e))?;
        Ok(Self {
            ws2812,
            wifi,
            ble,
            wifi_ssid: WIFI_SSID.to_string(),
            wifi_password: WIFI_PASSWD.to_string(),
        })
    }
    /// 连接wifi 传入 wifi 名称和密码
    pub fn wifi_connect(
        &mut self,
        wifi_ssid: String,
        wifi_passwd: String,
    ) -> Result<(), anyhow::Error> {
        let wifi = self.wifi.as_mut().ok_or(anyhow!("wifi unavailable"))?;
        if wifi.is_connected()? {
            log::info!("wifi is connected, now disconnecting");
            wifi.disconnect()?;
        }

        log::info!("wifi start");
        wifi.start()?;
        // 构造wifi名字和密码
        let mut ssid = heapless::String::<32>::new();
        let _ = ssid.push_str(wifi_ssid.as_str());
        let mut password = heapless::String::<64>::new();
        let _ = password.push_str(wifi_passwd.as_str());
        self.wifi_password = wifi_passwd;
        self.wifi_ssid = wifi_ssid;
        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid,
            password,
            auth_method: AuthMethod::WPA2Personal,
            ..Default::default()
        }))?;
        log::info!("wifi start connect");
        wifi.connect()?;
        Ok(())
    }

    pub fn wifi_ssid(&self) -> &str {
        &self.wifi_ssid
    }

    pub fn wifi_password(&self) -> &str {
        &self.wifi_password
    }
}
