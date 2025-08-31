use anyhow::{anyhow, Result};
use embedded_svc::wifi::Configuration;
use esp32_nimble::{
    uuid128, BLEAdvertisedDevice, BLEAdvertisementData, BLEDevice, BLEScan, NimbleProperties,
};
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::hal::task::block_on;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspNvsPartition,
    wifi::{AuthMethod, ClientConfiguration, EspWifi},
};
use std::thread;
use std::time::Duration;
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
        Self::ble_server_start()?;
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
                .start(ble, scan_time, |ble_device, _data| {
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

    fn ble_server_start() -> Result<(), anyhow::Error> {
        let ble = BLEDevice::take();
        let ble_advertising = ble.get_advertising();
        let server = ble.get_server();
        server.on_connect(|server, desc| {
            log::info!("Client connected: {:?}", desc);

            // 优化通信, 低功耗使用
            server
                .update_conn_params(desc.conn_handle(), 24, 48, 0, 60)
                .unwrap();

            // 没达到最大连接设备数就继续广播
            if server.connected_count() < (esp_idf_svc::sys::CONFIG_BT_NIMBLE_MAX_CONNECTIONS as _)
            {
                log::info!("Multi-connect support: start advertising");
                ble_advertising.lock().start().unwrap();
            }
        });

        server.on_disconnect(|_desc, reason| {
            log::info!("Disconnected from server: {:?}", reason);
        });
        let service = server.create_service(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa"));
        let static_characteristic = service.lock().create_characteristic(
            uuid128!("d4e0e0d0-1a2b-11e9-ab14-d663bd873d93"),
            NimbleProperties::READ,
        );
        static_characteristic
            .lock()
            .set_value(b"Hello World, this is static, TOTHTOT");

        // 通知特征, 能够向订阅这个uuid的设备不停发送消息
        let notifying_characteristic = service.lock().create_characteristic(
            uuid128!("a3c87500-8ed3-4bdf-8a39-a01bebede295"),
            NimbleProperties::READ | NimbleProperties::NOTIFY,
        );
        notifying_characteristic
            .lock()
            .set_value(b"Hello World, this is notify, TOTHTOT");

        // 写入特征, 通过这个uuid能够向esp发送数据
        let write_characteristic = service.lock().create_characteristic(
            uuid128!("3c9a3f00-8ed3-4bdf-8a39-a01bebede295"),
            NimbleProperties::READ | NimbleProperties::WRITE,
        );
        write_characteristic
            .lock()
            .on_read(move |characteristic, desc| {
                log::info!("characteristic: {:?}, {:?}", characteristic, desc);
            })
            .on_write(|args| {
                log::info!(
                    "wrote to write_characteristic: {:?} {:?}",
                    args.current_data(),
                    args.recv_data()
                );
            });

        // 设置蓝牙名称, 以及透传uuid, 开始蓝牙服务
        ble_advertising.lock().set_data(
            BLEAdvertisementData::new()
                .name("ESP32-GATT-Server")
                .add_service_uuid(uuid128!("fafafafa-fafa-fafa-fafa-fafafafafafa")),
        )?;
        ble_advertising.lock().start()?;

        // 开启连接日志显示
        server.ble_gatts_show_local();
        thread::spawn(move || {
            let mut counter = 0;
            loop {
                let notify_data = String::from(format!("Server run counter: {}", counter));
                log::info!("notify_data: {}", notify_data);
                notifying_characteristic
                    .lock()
                    .set_value(format!("Counter: {counter}").as_bytes())
                    .notify();
                counter += 1;
                thread::sleep(Duration::from_millis(1000));
            }
        });
        Ok(())
    }
    pub fn wifi_ssid(&self) -> &str {
        &self.wifi_ssid
    }

    pub fn wifi_password(&self) -> &str {
        &self.wifi_password
    }
}
