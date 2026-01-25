use crate::board::es8388::driver::Es8388;
use embedded_hal::digital::OutputPin;
use embedded_hal::i2c::I2c;

#[allow(dead_code)]
pub mod command;
pub mod driver;

/// 分段生成正弦波数据（复用缓冲区，避免重复分配内存）
/// freq: 频率
/// sample_rate: 采样率
/// chunk_ms: 每段时长
/// phase: 相位偏移
/// buffer: 复用的缓冲区
pub fn generate_sine_wave_chunk(
    freq: f32,
    sample_rate: f32,
    chunk_ms: u64,
    amplitude: f32,
    phase: &mut f32, // 确保这是一个可变引用
    buffer: &mut Vec<u8>,
) {
    buffer.clear();
    let num_samples = (sample_rate * chunk_ms as f32 / 1000.0) as usize;
    let step = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let max_amp = 32767.0 * amplitude.clamp(0.0, 1.0); // 使用 i32 范围

    for _ in 0..num_samples {
        let val = (phase.sin() * max_amp) as i16;
        let bytes = val.to_le_bytes(); // 4 字节

        // 写入左声道 (4字节)
        buffer.extend_from_slice(&bytes);
        // 写入右声道 (4字节)
        buffer.extend_from_slice(&bytes);

        *phase = (*phase + step).rem_euclid(2.0 * std::f32::consts::PI);
    }
}

pub fn generate_square_wave_chunk(
    freq: f32,
    sample_rate: f32,
    chunk_ms: u64,
    amplitude: f32,
    phase: &mut f32,
    buffer: &mut Vec<u8>,
) {
    buffer.clear();
    let num_samples = (sample_rate * chunk_ms as f32 / 1000.0) as usize;
    let step = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let max_amp = 32767.0 * amplitude.clamp(0.0, 1.0);

    for _i in 0..num_samples {
        // 判断当前相位在 [0, PI) 还是 [PI, 2PI)
        let val = if *phase < std::f32::consts::PI {
            max_amp as i16
        } else {
            -(max_amp as i16)
        };

        let bytes = val.to_le_bytes();
        buffer.extend_from_slice(&bytes); // Left Channel
        buffer.extend_from_slice(&bytes); // Right Channel
                                          // if i < 50 {
                                          //     log::info!("[{i}]: {:?}", bytes);
                                          // }
        *phase = (*phase + step).rem_euclid(2.0 * std::f32::consts::PI);
    }
}

pub fn play_sine_test(
    es8388: &mut Es8388<impl I2c, impl OutputPin>,
    freq: f32,
    duration_ms: u64,
) -> anyhow::Result<()> {
    let sample_rate = 44100.0;
    let chunk_ms = 20; // 每块 100ms 是平衡延迟和内存的最佳点
    let chunk_samples = (sample_rate * chunk_ms as f32 / 1000.0) as usize;

    // 提前分配内存：每个采样 4 字节 (16bit L+R)
    let mut audio_buffer = Vec::with_capacity(chunk_samples * 4);
    let mut phase = 0.0;

    // 确保 I2S 已开启
    es8388.i2s.tx_enable()?;

    // 确保功放开启
    es8388.set_speaker(true)?;

    let start_time = std::time::Instant::now();
    log::info!("开始播放正弦波测试音: {}Hz", freq);

    while start_time.elapsed().as_millis() < duration_ms as u128 {
        // generate_sine_wave_chunk(
        //     freq,
        //     sample_rate,
        //     chunk_ms,
        //     0.8, // 30% 的数字振幅，配合硬件音量 20-30 比较安全
        //     &mut phase,
        //     &mut audio_buffer,
        // );
        generate_square_wave_chunk(
            freq,
            sample_rate,
            chunk_ms,
            1.0, // 30% 的数字振幅，配合硬件音量 20-30 比较安全
            &mut phase,
            &mut audio_buffer,
        );

        es8388.write_audio(&audio_buffer, 200)?;
    }

    log::info!("播放结束");
    es8388.set_speaker(false)?;

    Ok(())
}
pub fn play_test_signal(es8388: &mut Es8388<impl I2c, impl OutputPin>) -> anyhow::Result<()> {
    es8388.i2s.tx_enable()?;
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

    for _ in 0..1000 {
        es8388.i2s.write(&buffer, 1000)?;
    }
    Ok(())
}
