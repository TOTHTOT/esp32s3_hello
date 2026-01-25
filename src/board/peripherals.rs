#[cfg(feature = "use_st7789")]
use crate::board::display;
use crate::board::es8388;
use crate::board::es8388::driver::{Es8388, RunMode};
use crate::board::file_system::init_fs;
use crate::board::share_i2c_bus::SharedI2cDevice;
use anyhow::Context;
use awedio::manager::Manager;
use core::cell::RefCell;
#[cfg(feature = "use_st7789")]
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use embedded_svc::wifi;
use esp32_nimble::{BLEAdvertisedDevice, BLEDevice, BLEScan};
use esp_idf_svc::hal::gpio::{InterruptType, Pull};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
use esp_idf_svc::hal::i2s::I2sDriver;
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::{
        gpio::{Gpio0, Gpio13, Gpio21, Output, PinDriver},
        prelude::*,
        spi::{SpiBusDriver, SpiDriver},
        task::block_on,
        temp_sensor::{TempSensorConfig, TempSensorDriver},
    },
    nvs::{EspNvsPartition, NvsDefault},
    wifi::{AuthMethod, EspWifi},
};
#[cfg(feature = "use_st7789")]
use mipidsi::interface::SpiInterface;
#[cfg(feature = "use_st7789")]
use mipidsi::models::ST7789;
#[cfg(feature = "use_st7789")]
use mipidsi::NoResetPin;
#[cfg(feature = "use_ws2812")]
use smart_leds::{
    hsv::{hsv2rgb, Hsv},
    SmartLedsWrite,
};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::{
    thread::{self},
    time::Duration,
};
#[cfg(feature = "use_ws2812")]
use ws2812_esp32_rmt_driver::lib_smart_leds::Ws2812Esp32Rmt;
use xl9555::driver::XL9555;

/// 默认连接的wifi
const WIFI_SSID: &str = "esp32_2.4G";
const WIFI_PASSWD: &str = "12345678..";

/// 屏幕引脚定义
#[cfg(feature = "use_st7789")]
type CsPin = PinDriver<'static, Gpio21, Output>;
#[cfg(feature = "use_st7789")]
type DcPin = PinDriver<'static, Gpio13, Output>;
#[allow(dead_code)]
type Xl9555PinType = Rc<RefCell<XL9555<SharedI2cDevice<Arc<Mutex<I2cDriver<'static>>>>>>>;
type Es8388Type = Es8388<SharedI2cDevice<Arc<Mutex<I2cDriver<'static>>>>>;
#[cfg(feature = "use_st7789")]
type MyDisplay = mipidsi::Display<
    SpiInterface<
        'static,
        ExclusiveDevice<SpiBusDriver<'static, SpiDriver<'static>>, CsPin, NoDelay>,
        DcPin,
    >,
    DisplayModel,
    NoResetPin,
>;

#[cfg(feature = "use_st7789")]
type DisplayModel = ST7789;
#[allow(dead_code)]
pub struct BspEsp32S3CoreBoard {
    #[cfg(feature = "use_ws2812")]
    pub ws2812: Ws2812Esp32Rmt<'static>,
    pub wifi: EspWifi<'static>,
    mcu_temperature: TempSensorDriver<'static>,
    fs_init: bool, // 标记文件系统是否初始化成功
    wifi_ssid: String,
    wifi_password: String,
    #[cfg(feature = "use_st7789")]
    display_rst_pin: xl9555::Pin,
    #[cfg(feature = "use_st7789")]
    display_backlight_pin: xl9555::Pin,
    #[cfg(feature = "use_st7789")]
    pub display: Option<MyDisplay>,
    pub xl9555: Xl9555PinType,
    pub es8388: Es8388Type,
    pub manager: Manager,
    // pub xl9555_interrupt: <PinDriver: PinDriver::InputPin +'static>,
}

#[derive(Default, Debug)]
pub struct BoardEsp32State {
    pub exit: bool,
    pub current_mcu_temperature: f32,
}

#[allow(dead_code)]
impl BspEsp32S3CoreBoard {
    pub fn new(peripherals: Peripherals, display_buf: &'static mut [u8]) -> anyhow::Result<Self> {
        let sys_loop = EspSystemEventLoop::take()?;
        let nvs = EspNvsPartition::<NvsDefault>::take()?;

        let mut fs_init = false;
        if init_fs().is_ok() {
            fs_init = true;
        }
        // 初始化spi, 这初始化能够让spi外设被多个设备使用
        let driver_config = Default::default();
        let spi_drv = SpiDriver::new(
            peripherals.spi2,
            peripherals.pins.gpio12,
            peripherals.pins.gpio11,
            None::<Gpio0>,
            &driver_config,
        )?;

        let wifi = EspWifi::new(peripherals.modem, sys_loop, Some(nvs.clone()))?;
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
        let i2c_bus = Arc::new(Mutex::new(i2c_driver));
        let mut xl9555 = XL9555::init(SharedI2cDevice(i2c_bus.clone()), (false, false, false));
        xl9555.xl9555_ioconfig(0b1111_0000_0000_0000)?;
        let xl9555_ref = Rc::new(RefCell::new(xl9555));

        let mut xl9555_interrupt = PinDriver::input(peripherals.pins.gpio0)?;
        xl9555_interrupt.set_pull(Pull::Up)?;
        xl9555_interrupt.set_interrupt_type(InterruptType::NegEdge)?;

        let es8388_i2c = SharedI2cDevice(i2c_bus.clone());
        let i2s = peripherals.i2s0;
        let mut es8388 = Es8388::new(es8388_i2c, es8388::driver::CHIP_ADDR, RunMode::AdcDac);
        // 初始化+启动
        es8388.init()?;
        es8388.start()?;
        let regs = es8388.read_all()?;
        log::info!("ES8388 Registers: ");
        regs.iter()
            .enumerate()
            .map(|(i, reg)| log::info!("reg[{}] = {:?}", i, reg))
            .last();

        let i2s_driver = I2sDriver::new_std_tx(
            i2s,
            &es8388::driver::default_i2s_config(),
            peripherals.pins.gpio46, // BCLK（ES8388的BCLK引脚）
            // peripherals.pins.gpio14,      // DIN（ES8388的ADCDAT引脚）
            peripherals.pins.gpio10,      // DOUT（ES8388的DACDAT引脚）
            Some(peripherals.pins.gpio3), // MCLK（必须！ES8388的MCLK引脚）
            peripherals.pins.gpio9,       // WS（ES8388的LRCK引脚）
        )
        .context("Failed to initialize I2S bidirectional driver")?;
        let backend = awedio_esp32::Esp32Backend::with_defaults(i2s_driver, 1, 44100, 128);
        let manager = backend.start();

        let mut board = Self {
            #[cfg(feature = "use_ws2812")]
            ws2812,
            wifi,
            mcu_temperature: temp_sensor,
            wifi_ssid: WIFI_SSID.to_string(),
            wifi_password: WIFI_PASSWD.to_string(),
            fs_init,
            #[cfg(feature = "use_st7789")]
            display: None,
            xl9555: xl9555_ref,
            #[cfg(feature = "use_st7789")]
            display_backlight_pin: xl9555::Pin::P13,
            #[cfg(feature = "use_st7789")]
            display_rst_pin: xl9555::Pin::P12,
            es8388,
            manager,
        };

        let spi_config =
            esp_idf_svc::hal::spi::SpiConfig::new().baudrate(FromValueType::MHz(30).into());
        let spi_bus_drv = SpiBusDriver::new(spi_drv, &spi_config)?;
        #[cfg(feature = "use_st7789")]
        {
            let _backlight_pin = xl9555::io::Output::new(
                &board.xl9555,
                board.display_backlight_pin,
                xl9555::PinState::High,
            );
            let _rst_pin = xl9555::io::Output::new(
                &board.xl9555,
                board.display_rst_pin,
                xl9555::PinState::High,
            );
            let display = display::new(
                spi_bus_drv,
                PinDriver::output(peripherals.pins.gpio21)?,
                PinDriver::output(peripherals.pins.gpio13)?,
                ST7789,
                display_buf,
                240,
                320,
            )?;
            board.display = Some(display);
        }
        log::info!("board init success");
        Ok(board)
    }

    /// 连接wifi 传入 wifi 名称和密码
    pub fn wifi_connect(&mut self) -> anyhow::Result<(), anyhow::Error> {
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
    pub fn rainbow_rgb(&mut self, hue: u8) -> anyhow::Result<()> {
        let pixels = std::iter::once(hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 8,
        }));
        self.ws2812.write(pixels)?;
        Ok(())
    }
    /// 扫描附近蓝牙, 并返回扫描结果类型: BLEAdvertisedDevice
    pub fn ble_scan(scan_time: i32) -> anyhow::Result<Vec<BLEAdvertisedDevice>, anyhow::Error> {
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
                    devices.push(*ble_device);
                    None::<BLEAdvertisedDevice>
                })
                .await
                .unwrap();
        });
        log::info!("Ble Scan end");
        for device in &devices {
            log::info!("device: {device:?}");
        }
        Ok(devices)
    }

    pub fn wifi_ssid(&self) -> &str {
        &self.wifi_ssid
    }

    pub fn wifi_password(&self) -> &str {
        &self.wifi_password
    }

    pub fn get_mcu_temperature(&mut self) -> anyhow::Result<f32> {
        let temp = self.mcu_temperature.get_celsius()?;
        Ok(temp)
    }

    pub fn get_fs_init(&self) -> bool {
        self.fs_init
    }

    pub fn set_fs_init(&mut self, fs_init: bool) {
        self.fs_init = fs_init;
    }

    /// 屏幕复位
    #[cfg(feature = "use_st7789")]
    pub fn display_rst(&self) -> anyhow::Result<()> {
        if self.display.is_none() {
            return Err(anyhow::Error::msg("display is none"));
        }
        self.xl9555
            .borrow_mut()
            .set_value(self.display_rst_pin, false)?;
        thread::sleep(Duration::from_micros(10));
        self.xl9555
            .borrow_mut()
            .set_value(self.display_rst_pin, true)?;
        Ok(())
    }
    /// 设置屏幕背光, 目前的显示屏的背光引脚有xl9555控制基本不支持pwm, 所以暂时用true和false控制
    #[cfg(feature = "use_st7789")]
    pub fn display_set_backlight(&self, backlight: u8) -> anyhow::Result<()> {
        if self.display.is_none() {
            return Err(anyhow::Error::msg("display is none"));
        }
        if backlight > 0 {
            self.xl9555
                .borrow_mut()
                .set_value(self.display_backlight_pin, true)?;
        } else {
            self.xl9555
                .borrow_mut()
                .set_value(self.display_backlight_pin, false)?;
        }
        Ok(())
    }
}
