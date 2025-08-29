use anyhow::{anyhow, Result};
use esp_idf_svc::hal::prelude::Peripherals;
use ws2812_esp32_rmt_driver::lib_smart_leds::Ws2812Esp32Rmt;

pub struct BspEsp32S3CoreBoard<'d> {
    pub ws2812: Ws2812Esp32Rmt<'d>,
}

impl<'d> BspEsp32S3CoreBoard<'d> {
    pub fn new(peripherals: Peripherals) -> Result<Self> {
        let ws2812 = Ws2812Esp32Rmt::new(peripherals.rmt.channel0, peripherals.pins.gpio48)
            .map_err(|e| anyhow!("Ws2812Esp32Rmt error: {:?}", e))?;

        Ok(Self { ws2812 })
    }
}
