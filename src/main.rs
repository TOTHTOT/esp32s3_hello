mod board;

use board::BspEsp32S3CoreBoard;
use esp_idf_svc::hal::peripherals::Peripherals;
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::SmartLedsWrite;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let mut board = BspEsp32S3CoreBoard::new(peripherals)?;
    board.wifi_connect("RAYNEN".to_string(), "RN603933".to_string())?;
    // 有需要的话可以在线程结束后回收
    let _ble_server_handle = board.ble_server_start()?;
    let _http_server_handle = board.test_http_server()?;

    let mut hue: u8 = 0;

    loop {
        let pixels = std::iter::once(hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 8,
        }));
        if let Err(e) = board.ws2812.write(pixels) {
            log::error!("Ws2812 write error:{e}");
        }
        board.get_mcu_temperature()?;
        thread::sleep(Duration::from_millis(50));
        hue = hue.wrapping_add(10);
    }
}
