// 错误处理
use anyhow::{anyhow, Result};

// 显示屏相关
use crate::display;
// 嵌入式服务与协议
use core::cell::RefCell;
// 标准库
use embedded_graphics::pixelcolor::Rgb565;
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use embedded_svc::wifi;
// BLE相关
use esp32_nimble::{
    uuid128, BLEAdvertisedDevice, BLEAdvertisementData, BLEDevice, BLEScan, NimbleProperties,
};
use std::rc::Rc;
use std::{
    ffi::CString,
    fs::{File, OpenOptions},
    io::{Read as StdRead, Write as StdWrite},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

// ESP-IDF核心服务与硬件抽象
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{Gpio0, Gpio15, Gpio16, Gpio21, Output, PinDriver},
        prelude::*,
        spi::{SpiBusDriver, SpiDriver},
        task::block_on,
        temp_sensor::{TempSensorConfig, TempSensorDriver},
    },
    nvs::{EspNvsPartition, NvsDefault},
    sys,
    sys::{
        esp, esp_vfs_fat_mount_config_t, esp_vfs_fat_spiflash_mount, nvs_flash_erase,
        nvs_flash_init, wl_handle_t, ESP_ERR_NVS_NEW_VERSION_FOUND, ESP_ERR_NVS_NO_FREE_PAGES,
    },
    wifi::{AuthMethod, EspWifi},
};
use mipidsi::interface::{Interface, InterfacePixelFormat, SpiInterface};
use mipidsi::models::ST7789;
use mipidsi::NoResetPin;
// WS2812 LED驱动
#[cfg(feature = "use_ws2812")]
use smart_leds::{
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite,
};
#[cfg(feature = "use_ws2812")]
use ws2812_esp32_rmt_driver::lib_smart_leds::Ws2812Esp32Rmt;
use xl9555::driver::XL9555;

/// 默认连接的wifi
const WIFI_SSID: &str = "esp32_2.4G";
const WIFI_PASSWD: &str = "12345678..";

/// 屏幕引脚定义
type CsPin<'d> = PinDriver<'d, Gpio21, Output>;
type DcPin<'d> = xl9555::io::Output<'d, I2cDriver<'d>>;
// #[allow(dead_code)]
type RstPin<'d> = xl9555::io::Output<'d, I2cDriver<'d>>;
type DisplayModel = ST7789;
pub struct BspEsp32S3CoreBoard<'d>
where
// MODEL: Model<ColorFormat = Rgb565>,
{
    #[cfg(feature = "use_ws2812")]
    pub ws2812: Ws2812Esp32Rmt<'d>,
    pub wifi: EspWifi<'d>,
    mcu_temperature: TempSensorDriver<'d>,
    fs_init: bool, // 标记文件系统是否初始化成功
    wifi_ssid: String,
    wifi_password: String,
    pub display: Option<
        mipidsi::Display<
            SpiInterface<
                'd,
                ExclusiveDevice<SpiBusDriver<'d, SpiDriver<'d>>, CsPin<'d>, NoDelay>,
                DcPin<'d>,
            >,
            DisplayModel,
            NoResetPin,
        >,
    >,

    pub xl9555: RefCell<XL9555<I2cDriver<'d>>>,
}

#[derive(Default, Debug)]
pub struct BoardEsp32State {
    pub exit: bool,
    pub current_mcu_temperature: f32,
}

#[allow(dead_code)]
impl<'d> BspEsp32S3CoreBoard<'d>
where
    // MODEL: Model<ColorFormat = Rgb565>,
    Rgb565: InterfacePixelFormat<
        <SpiInterface<
            'd,
            ExclusiveDevice<SpiBusDriver<'d, SpiDriver<'d>>, CsPin<'d>, NoDelay>,
            DcPin<'d>,
        > as Interface>::Word,
    >,
{
    pub fn new(peripherals: Peripherals, display_buf: &'d mut [u8]) -> Result<Self> {
        let sysloop = EspSystemEventLoop::take()?;
        let nvs = EspNvsPartition::<NvsDefault>::take()?;

        let mut fs_init = false;
        if let Ok(_) = BspEsp32S3CoreBoard::init_fs() {
            fs_init = true;
        }
        // 初始化spi, 这初始化能够让spi外设被多个设备使用
        let driver_config = Default::default();
        let spi_drv = SpiDriver::new(
            peripherals.spi2,
            peripherals.pins.gpio18,
            peripherals.pins.gpio17,
            None::<Gpio0>,
            &driver_config,
        )?;

        let wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs.clone()))?;
        log::info!("start init ws2812");
        #[cfg(feature = "use_ws2812")]
        let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio48)
            .map_err(|e| anyhow!("Ws2812Esp32Rmt error: {:?}", e))?;

        let mut temp_sensor =
            TempSensorDriver::new(&TempSensorConfig::default(), peripherals.temp_sensor)?;
        temp_sensor.enable()?;

        let i2c_driver = I2cDriver::new(
            peripherals.i2c0,
            peripherals.pins.gpio41,
            peripherals.pins.gpio42,
            &I2cConfig::new().baudrate(FromValueType::kHz(100).into()),
        )?;
        let mut xl9555 = XL9555::init(i2c_driver, (false, false, false));
        xl9555.xl9555_ioconfig(0b1111_0000_0000_0000)?;
        let xl9555_ref = RefCell::new(xl9555);

        let spi_config =
            esp_idf_svc::hal::spi::SpiConfig::new().baudrate(FromValueType::MHz(30).into());
        let spi_buf = SpiBusDriver::new(spi_drv, &spi_config)?;
        let model = ST7789;
        let dc_pin = xl9555::io::Output::new(&xl9555_ref, xl9555::Pin::P13, xl9555::PinState::Low);
        let rst_pin = xl9555::io::Output::new(&xl9555_ref, xl9555::Pin::P12, xl9555::PinState::Low);
        let display = display::new(
            spi_buf,
            PinDriver::output(peripherals.pins.gpio21)?,
            dc_pin,  // 实际上xl9555的io13
            rst_pin, // 实际上xl9555的io12
            model,
            display_buf,
        )?;
        let mut board = Self {
            #[cfg(feature = "use_ws2812")]
            ws2812,
            wifi,
            mcu_temperature: temp_sensor,
            wifi_ssid: WIFI_SSID.to_string(),
            wifi_password: WIFI_PASSWD.to_string(),
            fs_init,
            display: Some(display),
            xl9555: xl9555_ref,
        };
        board.wifi_connect()?;
        log::info!("board init success");
        Ok(board)
    }

    fn init_fs() -> Result<()> {
        log::info!("init_fs");
        unsafe {
            let ret = nvs_flash_init();
            if ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND {
                log::info!("fat partition need init");
                // 如果 nvs 需要擦除
                nvs_flash_erase();
                nvs_flash_init();
            } else {
                esp!(ret)?;
            }
        }

        // 启用磨损均衡功能
        let mut wl_handle = 0;
        let mount_config = esp_vfs_fat_mount_config_t {
            max_files: 5,
            format_if_mount_failed: true,
            allocation_unit_size: 4096,

            disk_status_check_enable: false,
            use_one_fat: false,
        };

        // 挂载 FAT 到 /fat（分区 label 必须与 partitions.csv 中一致.
        let mount_point = String::from("/fat");
        let partition_label = String::from("storage");
        let res = unsafe {
            // 和c交互只能使用CString.
            esp_vfs_fat_spiflash_mount(
                CString::new(mount_point)?.as_ptr(),
                CString::new(partition_label)?.as_ptr(),
                &mount_config,
                &mut wl_handle as *mut wl_handle_t,
            )
        };

        if res != sys::ESP_OK {
            log::error!("esp_vfs_fat_spiflash_mount failed: {}", res);
            return Err(anyhow!(res));
        }
        log::info!("FAT mounted at /fat");
        Self::test_fs_rw()?;
        Ok(())
    }

    /// `test_fs_rw` 测试文件系统读写
    fn test_fs_rw() -> Result<()> {
        let path = "/fat/hello.txt";
        {
            let mut f = OpenOptions::new()
                .create(true)
                .write(true)
                .open(path)
                .expect("create file failed");
            f.write_all(b"hello from rust on esp32!\n")?;
        }
        let mut s = String::new();
        let mut f = File::open(path)?;
        f.read_to_string(&mut s)?;
        log::info!("file content: {}", s);
        Ok(())
    }

    /// 连接wifi 传入 wifi 名称和密码
    pub fn wifi_connect(&mut self) -> Result<(), anyhow::Error> {
        if self.wifi.is_connected()? {
            log::info!("wifi is connected, now disconnecting");
            self.wifi.disconnect()?;
        }

        log::info!("wifi start");
        self.wifi.start()?;
        // 构造wifi名字和密码
        let mut ssid = heapless::String::<32>::new();
        let _ = ssid.push_str(self.wifi_ssid.as_str());
        let mut password = heapless::String::<64>::new();
        let _ = password.push_str(self.wifi_password.as_str());
        self.wifi
            .set_configuration(&wifi::Configuration::Client(wifi::ClientConfiguration {
                ssid,
                password,
                auth_method: AuthMethod::WPA2Personal,
                ..Default::default()
            }))?;
        #[cfg(feature = "enable_wifi_scan")]
        {
            log::info!("wifi scan start");
            let scan = self.wifi.scan()?;
            for rr in scan {
                log::info!("scan: {:?}", rr);
            }
        }
        log::info!("wifi start connect, ");
        self.wifi.connect()?;
        Ok(())
    }
    #[cfg(feature = "use_ws2812")]
    pub fn rainbow_rgb(&mut self, hue: u8) -> Result<()> {
        let pixels = std::iter::once(hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 8,
        }));
        self.ws2812.write(pixels)?;
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

    pub fn ble_server_start(
        board: Arc<Mutex<BoardEsp32State>>,
    ) -> Result<JoinHandle<Result<()>>, anyhow::Error> {
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

        let handle = thread::spawn(move || -> Result<()> {
            let mut counter = 0;
            loop {
                thread::sleep(Duration::from_millis(1000));
                let board_state = board.lock().expect("Failed to lock board mutex");
                let temp = board_state.current_mcu_temperature;
                if board_state.exit == true {
                    log::info!("ble server stopped");
                    break Ok(());
                }
                let notify_str = String::from(format!("running:{counter},temp:{temp}",));
                // log::info!("{notify_str}");
                notifying_characteristic
                    .lock()
                    .set_value(notify_str.as_bytes())
                    .notify();
                counter += 1;
            }
        });
        Ok(handle)
    }

    pub fn wifi_ssid(&self) -> &str {
        &self.wifi_ssid
    }

    pub fn wifi_password(&self) -> &str {
        &self.wifi_password
    }

    pub fn get_mcu_temperature(&mut self) -> Result<f32> {
        let temp = self.mcu_temperature.get_celsius()?;
        Ok(temp)
    }

    pub fn get_fs_init(&self) -> bool {
        self.fs_init
    }

    pub fn set_fs_init(&mut self, fs_init: bool) {
        self.fs_init = fs_init;
    }
}
