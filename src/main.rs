use std::thread;
use std::time::Duration;
use esp_idf_svc::hal::peripherals::Peripherals;
use smart_leds::hsv::{hsv2rgb, Hsv};
use ws2812_esp32_rmt_driver::lib_smart_leds::Ws2812Esp32Rmt;
use smart_leds::SmartLedsWrite;

fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();

    esp_idf_svc::log::EspLogger::initialize_default();
    let peripherals = Peripherals::take()?;
    let pin = peripherals.pins.gpio48;
    let channel = peripherals.rmt.channel0;
    let mut ws2812 = Ws2812Esp32Rmt::new(channel, pin)?;

    let mut hue:u8 = 0;
    loop {
        let pixels = std::iter::once(hsv2rgb(Hsv{
            hue, sat: 255, val: 8
        }));
        if let Err(e) = ws2812.write(pixels) {
            log::error!("Ws2812 write error:{e}");
        }
        thread::sleep(Duration::from_millis(500));
        hue = hue.wrapping_add(10);
    }
}
