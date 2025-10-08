mod board;
mod display;
mod http_server;

use crate::board::BoardEsp32State;
use board::BspEsp32S3CoreBoard;
use esp_idf_svc::hal::peripherals::Peripherals;
use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::SmartLedsWrite;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    let mut board = BspEsp32S3CoreBoard::new(peripherals)?;
    let board_state = BoardEsp32State::default();
    // 有需要的话可以在线程结束后回收
    let board_http = Arc::new(Mutex::new(board_state));
    let board_ble = Arc::clone(&board_http);
    let board_state = Arc::clone(&board_http);
    let _ble_server_handle = BspEsp32S3CoreBoard::ble_server_start(board_ble)?;
    let _http_server_handle = http_server::HttpServer::new(board_http)?;
    let mut hue: u8 = 0;
    let mut loop_times = 0;
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
        if loop_times % 100 == 0 {
            log::info!("board status:{board_state:?}");
        }
        loop_times += 1;
    }
}
