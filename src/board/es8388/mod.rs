#[allow(dead_code)]
pub mod command;
pub mod driver;

use awedio::manager::Manager;
use awedio::sounds::MemorySound;
use hound::WavReader;
use std::io::Cursor;
use std::sync::Arc;
const WAV_DATA: &[u8] = include_bytes!("../../../assets/tools/test_audio_44.1kHz.wav");

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

// struct StaticRawSound {
//     data: &'static [i16],
//     pos: usize,
// }
//
// impl awedio::Sound for StaticRawSound {
//     fn channel_count(&self) -> u16 { 1 } // 根据你转换的设置修改
//     fn sample_rate(&self) -> u32 { 44100 }
//     fn next_sample(&mut self) -> Result<awedio::NextSample, awedio::Error> {
//         if self.pos < self.data.len() {
//             let s = self.data[self.pos];
//             self.pos += 1;
//             Ok(awedio::NextSample::Sample(s))
//         } else {
//             Ok(awedio::NextSample::Finished)
//         }
//     }
//
//     fn on_start_of_batch(&mut self) {
//         todo!()
//     }
// }

pub fn play_wav(manager: &mut Manager) {
    // 1. 使用 hound 读取 wav 格式

    let mut reader = WavReader::new(Cursor::new(WAV_DATA)).expect("无效的 WAV 格式");
    let spec = reader.spec();

    // 2. 提取采样点
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

    // 3. 创建 MemorySound
    // 注意：MemorySound::new 的参数顺序通常为 (samples, sample_rate, channels)
    let sound = MemorySound::from_samples(
        Arc::new(samples),
        spec.channels,    // 源码中要求 u16
        spec.sample_rate, // 源码中要求 u32
    );

    // 4. 包装并播放
    manager.play(Box::new(sound));
}
