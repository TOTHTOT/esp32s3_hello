use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    image::{Image, ImageRaw, ImageRawLE},
    pixelcolor::Rgb565,
    prelude::RgbColor,
    Drawable,
};
use embedded_hal_bus::spi::{ExclusiveDevice, NoDelay};
use esp_idf_svc::hal::spi::SpiBusDriver;
use esp_idf_svc::hal::{delay::FreeRtos, spi::SpiDriver};
use mipidsi::{
    interface::{Interface, InterfacePixelFormat, SpiInterface},
    models::Model,
    NoResetPin,
    {options::ColorInversion, Builder},
};

pub fn new<'d, DC, RST, CS, MODEL>(
    spi: SpiBusDriver<'d, SpiDriver<'d>>,
    cs: CS,
    dc: DC,
    rst: RST,
    model: MODEL,
    buffer: &'d mut [u8],
    width: u16,
    height: u16,
) -> anyhow::Result<
    mipidsi::Display<
        SpiInterface<'d, ExclusiveDevice<SpiBusDriver<'d, SpiDriver<'d>>, CS, NoDelay>, DC>,
        MODEL,
        NoResetPin,
    >,
>
    where
        CS: embedded_hal::digital::OutputPin,
        DC: embedded_hal::digital::OutputPin,
        RST: embedded_hal::digital::OutputPin,
        MODEL: Model<ColorFormat=Rgb565>,
        Rgb565: InterfacePixelFormat<
            <SpiInterface<'d, ExclusiveDevice<SpiBusDriver<'d, SpiDriver<'d>>, CS, NoDelay>, DC> as Interface>::Word
        >,
    {
    let spi_device = ExclusiveDevice::new_no_delay(spi, cs).unwrap();
    let _rst = rst;
    let di = SpiInterface::new(spi_device, dc, buffer);
    let mut delay = FreeRtos;
    let mut display = Builder::new(model, di)
        .display_size(width, height)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();
    display.clear(Rgb565::WHITE).unwrap();

    let image_raw: ImageRawLE<Rgb565> = ImageRaw::new(include_bytes!("../assets/ferris.raw"), 86);
    let image = Image::new(&image_raw, Point::new(26, 8));
    image.draw(&mut display).unwrap();
    Ok(display)
}
