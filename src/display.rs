#[cfg(feature = "use_st7735")]
pub mod st7735_display {
    use embedded_graphics::{
        draw_target::DrawTarget,
        geometry::Point,
        image::{Image, ImageRaw, ImageRawLE},
        pixelcolor::Rgb565,
        prelude::RgbColor,
        Drawable,
    };

    use esp_idf_svc::hal::{
        delay::FreeRtos,
        gpio::{InputPin, OutputPin},
        peripheral::Peripheral,
        prelude::*,
        spi::{SpiDeviceDriver, SpiDriver},
    };

    pub use st7735_lcd::{Orientation, ST7735};
    pub fn display_init<'d, DC, RST>(
        spi: impl Peripheral<P = impl esp_idf_svc::hal::spi::SpiAnyPins> + 'd,
        sclk: impl Peripheral<P = impl OutputPin> + 'd,
        sdo: impl Peripheral<P = impl OutputPin> + 'd,
        sdi: Option<impl Peripheral<P = impl InputPin> + 'd>,
        cs: Option<impl Peripheral<P = impl OutputPin> + 'd>,
        dc: DC,
        rst: RST,
    ) -> anyhow::Result<ST7735<SpiDeviceDriver<'d, SpiDriver<'d>>, DC, RST>>
    where
        DC: embedded_hal::digital::OutputPin,
        RST: embedded_hal::digital::OutputPin,
    {
        let driver_config = Default::default();
        let spi_config =
            esp_idf_svc::hal::spi::SpiConfig::new().baudrate(FromValueType::MHz(30).into());
        let spi_dd = esp_idf_svc::hal::spi::SpiDeviceDriver::new_single(
            spi,
            sclk,
            sdo,
            sdi,
            cs,
            &driver_config,
            &spi_config,
        )?;
        let rgb = true;
        let inverted = false;
        let width = 128;
        let height = 160;

        let mut display = ST7735::new(spi_dd, dc, rst, rgb, inverted, width, height);
        let mut delay = FreeRtos;

        display.init(&mut delay).unwrap();
        display.clear(Rgb565::BLACK).unwrap();
        display
            .set_orientation(&Orientation::LandscapeSwapped)
            .unwrap();
        display.set_offset(0, 25);
        let image_raw: ImageRawLE<Rgb565> =
            ImageRaw::new(include_bytes!("../assets/ferris.raw"), 86);
        let image = Image::new(&image_raw, Point::new(26, 8));
        image.draw(&mut display).unwrap();
        Ok(display)
    }
}

#[cfg(feature = "use_st7789")]
pub mod st7789_display {
    pub fn display_init() {}
}
