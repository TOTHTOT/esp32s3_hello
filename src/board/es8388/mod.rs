#[allow(dead_code)]
pub mod command;
pub mod driver;

#[allow(dead_code)]
pub fn play_test_signal() -> anyhow::Result<Vec<u8>> {
    let mut buffer = Vec::with_capacity(4096);
    let amplitude: i16 = 20000; // 稍微降低一点防止削波噪声

    for i in 0..2048 {
        let val = if (i / 1024) % 2 == 0 {
            amplitude
        } else {
            -amplitude
        };
        let bytes = val.to_le_bytes();
        buffer.extend_from_slice(&bytes);
        buffer.extend_from_slice(&bytes);
    }
    Ok(buffer)
}
