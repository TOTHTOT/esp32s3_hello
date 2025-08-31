use anyhow::{anyhow, Result};
use embedded_svc::wifi::Configuration;
use esp32_nimble::{uuid128, BLEAdvertisedDevice, BLEDevice, BLEScan};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::{
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
    pub wifi: EspWifi<'d>,
    wifi_ssid: String,
    wifi_password: String,
}

#[allow(dead_code)]
impl<'d> BspEsp32S3CoreBoard<'d> {
    pub fn new(peripherals: Peripherals) -> Result<Self> {
        let sysloop = EspSystemEventLoop::take()?;
        let nvs = EspNvsPartition::<NvsDefault>::take()?;

        let wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs.clone()))?;
        Self::ble_scan(10000)?;
        log::info!("start init ws2812");
        let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio48)
            .map_err(|e| anyhow!("Ws2812Esp32Rmt error: {:?}", e))?;
        Ok(Self {
            ws2812,
            wifi,
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
        if self.wifi.is_connected()? {
            log::info!("wifi is connected, now disconnecting");
            self.wifi.disconnect()?;
        }

        log::info!("wifi start");
        self.wifi.start()?;
        // 构造wifi名字和密码
        let mut ssid = heapless::String::<32>::new();
        let _ = ssid.push_str(wifi_ssid.as_str());
        let mut password = heapless::String::<64>::new();
        let _ = password.push_str(wifi_passwd.as_str());
        self.wifi_password = wifi_passwd;
        self.wifi_ssid = wifi_ssid;
        self.wifi
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid,
                password,
                auth_method: AuthMethod::WPA2Personal,
                ..Default::default()
            }))?;
        log::info!("wifi start connect");
        self.wifi.connect()?;
        Ok(())
    }
    /// 扫描附近蓝牙, 并返回扫描结果类型: BLEAdvertisedDevice
    pub fn ble_scan(scan_time: i32) -> Result<Vec<BLEAdvertisedDevice>, anyhow::Error> {
        let ble = BLEDevice::take();
        let mut ble_scan = BLEScan::new();
        let mut devices = Vec::new();
        log::info!("BleScanning...");
        block_on(async {
            ble_scan
                .active_scan(true)
                .interval(1000)
                .window(99)
                .start(ble, scan_time, |ble_device, data| {
                    devices.push(ble_device.clone());
                    None::<BLEAdvertisedDevice>
                })
                .await
                .unwrap();
        });
        log::info!("Ble Scan end");
        for device in &devices {
            log::info!("device: {:?}", device);
        }
        Ok(devices)
    }

    pub fn wifi_ssid(&self) -> &str {
        &self.wifi_ssid
    }

    pub fn wifi_password(&self) -> &str {
        &self.wifi_password
    }
}
