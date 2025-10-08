use embedded_graphics::draw_target::DrawTarget;
use embedded_graphics::geometry::Point;
use embedded_graphics::image::{Image, ImageRaw, ImageRawLE};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RgbColor;
use embedded_graphics::Drawable;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{AnyOutputPin, InputPin, Output, OutputPin, PinDriver};
use esp_idf_svc::hal::peripheral::Peripheral;
use esp_idf_svc::hal::prelude::*;
use esp_idf_svc::hal::spi;
use esp_idf_svc::hal::spi::{SpiAnyPins, SpiDeviceDriver};
use st7735_lcd::{Orientation, ST7735};
pub fn display_init<'d, DC, RST>(
    spi: impl Peripheral<P = spi::SPI2> + 'd,
    sclk: impl Peripheral<P = impl OutputPin> + 'd,
    sdo: impl Peripheral<P = impl OutputPin> + 'd,
    sdi: Option<impl Peripheral<P = impl InputPin> + 'd>,
    cs: Option<impl Peripheral<P = impl OutputPin> + 'd>,
    dc: impl Peripheral<P = DC> + 'd,
    rst: impl Peripheral<P = RST> + 'd,
) -> anyhow::Result<
    ST7735<
        SpiDeviceDriver<'d, spi::SpiDriver<'d>>,
        PinDriver<'d, DC, Output>,
        PinDriver<'d, RST, Output>,
    >,
>
where
    DC: OutputPin + 'd,
    RST: OutputPin + 'd,
{
    let driver_config = Default::default();
    let spi_config = spi::SpiConfig::new().baudrate(FromValueType::MHz(30).into());
    let spi =
        spi::SpiDeviceDriver::new_single(spi, sclk, sdo, sdi, cs, &driver_config, &spi_config)?;

    let rgb = false;
    let inverted = false;
    let width = 128;
    let height = 160;

    let mut display = ST7735::new(
        spi,
        PinDriver::output(dc)?,
        PinDriver::output(rst)?,
        rgb,
        inverted,
        width,
        height,
    );
    let mut delay = FreeRtos;

    display.init(&mut delay).unwrap();
    display.clear(Rgb565::BLACK).unwrap();
    display
        .set_orientation(&Orientation::LandscapeSwapped)
        .unwrap();
    display.set_offset(0, 25);
    let image_raw: ImageRawLE<Rgb565> = ImageRaw::new(include_bytes!("../assets/ferris.raw"), 86);
    let image = Image::new(&image_raw, Point::new(26, 8));
    image.draw(&mut display).unwrap();
    Ok(display)
}
