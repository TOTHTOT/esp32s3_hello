mod board;
mod http_server;

use crate::board::es8388;
use board::peripherals::BoardEsp32State;
use board::peripherals::BspEsp32S3CoreBoard;
use embedded_hal::digital::PinState;
use esp_idf_svc::hal::peripherals::Peripherals;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take()?;
    // let mut display_buffer = [0_u8; 512];
    let display_buffer: &'static mut [u8] = Box::leak(Box::new([0_u8; 512]));
    let mut board = BspEsp32S3CoreBoard::new(peripherals, display_buffer)?;
    let board_state = BoardEsp32State::default();
    // 有需要的话可以在线程结束后回收
    let board_http = Arc::new(Mutex::new(board_state));
    // let board_ble = Arc::clone(&board_http);
    let board_state = Arc::clone(&board_http);
    // let _ble_server_handle = board::ble::ble_server_start(board_ble)?;
    // let _http_server_handle = http_server::HttpServer::new(board_http)?;

    let _ =
        xl9555::io::Output::new(&board.xl9555.clone(), xl9555::Pin::P02, PinState::Low).set_low();
    let wave = awedio::sounds::SineWave::new(840.0);
    board.manager.play(Box::new(wave));
    thread::sleep(Duration::from_millis(5000));
    board.manager.clear();

    es8388::play_wav(&mut board.manager);
    let mut loop_times = 0;
    #[cfg(feature = "use_ws2812")]
    let mut hue: u8 = 0;
    loop {
        thread::sleep(Duration::from_millis(50));
        let mut state = board_state.lock().expect("Could not lock board state");
        state.current_mcu_temperature = board.get_mcu_temperature()?;
        #[cfg(feature = "use_ws2812")]
        {
            hue = hue.wrapping_add(10);
            board.rainbow_rgb(hue)?;
        }
        if loop_times % 100 == 0 {
            // let cur_pin_state = board.xl9555.read_value(xl9555::Pin::P03)?;
            // board.xl9555.set_value(xl9555::Pin::P03, !cur_pin_state)?;
            log::info!(
                "board status:{state:?}\nall_pin_state = {:016b}",
                board.xl9555.borrow_mut().read_all_value()?
            );
        }
        loop_times += 1;
    }
}
