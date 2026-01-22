#[cfg(feature = "use_st7789")]
use crate::board::display;
use crate::board::file_system::init_fs;
use core::cell::RefCell;
#[cfg(feature = "use_st7789")]
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use embedded_svc::wifi;
use esp32_nimble::{BLEAdvertisedDevice, BLEDevice, BLEScan};
use esp_idf_svc::hal::gpio::{InterruptType, Pull};
use esp_idf_svc::hal::i2c::{I2cConfig, I2cDriver};
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
type CsPin<'d> = PinDriver<'d, Gpio21, Output>;
#[cfg(feature = "use_st7789")]
type DcPin<'d> = PinDriver<'d, Gpio13, Output>;
#[allow(dead_code)]
type Xl9555Pin<'d> = xl9555::io::Output<'d, I2cDriver<'d>>;
#[cfg(feature = "use_st7789")]
type MyDisplay<'d> = mipidsi::Display<
    SpiInterface<
        'd,
        ExclusiveDevice<SpiBusDriver<'d, SpiDriver<'d>>, CsPin<'d>, NoDelay>,
        DcPin<'d>,
    >,
    DisplayModel,
    NoResetPin,
>;

#[cfg(feature = "use_st7789")]
type DisplayModel = ST7789;
pub struct BspEsp32S3CoreBoard<'d> {
    #[cfg(feature = "use_ws2812")]
    pub ws2812: Ws2812Esp32Rmt<'d>,
    pub wifi: EspWifi<'d>,
    mcu_temperature: TempSensorDriver<'d>,
    fs_init: bool, // 标记文件系统是否初始化成功
    wifi_ssid: String,
    wifi_password: String,
    #[cfg(feature = "use_st7789")]
    display_rst_pin: xl9555::Pin,
    #[cfg(feature = "use_st7789")]
    display_backlight_pin: xl9555::Pin,
    #[cfg(feature = "use_st7789")]
    pub display: Option<MyDisplay<'d>>,
    pub xl9555: Rc<RefCell<XL9555<I2cDriver<'d>>>>,
    // pub xl9555_interrupt: <PinDriver: PinDriver::InputPin +'d>,
}

#[derive(Default, Debug)]
pub struct BoardEsp32State {
    pub exit: bool,
    pub current_mcu_temperature: f32,
}

#[allow(dead_code)]
impl<'d> BspEsp32S3CoreBoard<'d> {
    pub fn new(peripherals: Peripherals, display_buf: &'d mut [u8]) -> anyhow::Result<Self> {
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
        let mut xl9555 = XL9555::init(i2c_driver, (false, false, false));
        xl9555.xl9555_ioconfig(0b1111_0000_0000_0000)?;
        let xl9555_ref = Rc::new(RefCell::new(xl9555));

        let mut xl9555_interrupt = PinDriver::input(peripherals.pins.gpio0)?;
        xl9555_interrupt.set_pull(Pull::Up)?;
        xl9555_interrupt.set_interrupt_type(InterruptType::NegEdge)?;

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
