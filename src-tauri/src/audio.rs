use std::{
    fs::File,
    io::BufWriter,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
};
use hound::{SampleFormat as WavSampleFormat, WavSpec, WavWriter};
use uuid::Uuid;

use crate::{AppError, AppResult};

type SharedWriter = Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>;
pub type LevelCallback = Arc<dyn Fn(f32) + Send + Sync + 'static>;

pub struct RecordingSession {
    pub path: PathBuf,
    started_at: Instant,
    stream: Stream,
    writer: SharedWriter,
}

pub struct StoppedRecording {
    pub path: PathBuf,
    pub duration_seconds: f64,
}

// MVP note: the active stream is only accessed through the Tauri-managed mutex
// to stop/drop it. A later macOS hardening pass should move recording onto a
// dedicated audio thread instead of storing the stream directly in app state.
unsafe impl Send for RecordingSession {}

impl RecordingSession {
    pub fn stop(self) -> AppResult<StoppedRecording> {
        let duration_seconds = self.started_at.elapsed().as_secs_f64();
        drop(self.stream);
        let mut guard = self
            .writer
            .lock()
            .map_err(|_| AppError::Audio("录音写入器锁定失败。".to_string()))?;
        if let Some(writer) = guard.take() {
            writer.finalize()?;
        }
        Ok(StoppedRecording {
            path: self.path,
            duration_seconds,
        })
    }
}

pub fn start(level_callback: LevelCallback) -> AppResult<RecordingSession> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| AppError::Audio("没有找到可用的麦克风输入设备。".to_string()))?;
    let supported_config = device.default_input_config()?;
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();

    let path = std::env::temp_dir().join(format!("glorious-evolution-{}.wav", Uuid::new_v4()));
    let spec = WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate.0,
        bits_per_sample: 16,
        sample_format: WavSampleFormat::Int,
    };
    let writer = Arc::new(Mutex::new(Some(WavWriter::create(&path, spec)?)));

    let stream = match sample_format {
        SampleFormat::I8 => build_stream::<i8>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::I32 => build_stream::<i32>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::I64 => build_stream::<i64>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::U8 => build_stream::<u8>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::U32 => build_stream::<u32>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::U64 => build_stream::<u64>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::F32 => build_stream::<f32>(&device, &config, writer.clone(), level_callback.clone())?,
        SampleFormat::F64 => build_stream::<f64>(&device, &config, writer.clone(), level_callback.clone())?,
        sample => {
            return Err(AppError::Audio(format!(
                "暂不支持当前麦克风采样格式：{sample:?}"
            )))
        }
    };

    stream.play()?;
    Ok(RecordingSession {
        path,
        started_at: Instant::now(),
        stream,
        writer,
    })
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    writer: SharedWriter,
    level_callback: LevelCallback,
) -> AppResult<Stream>
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let err_fn = |error| eprintln!("audio input stream error: {error}");
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            let mut sum_squares = 0.0f32;
            let mut count = 0usize;
            if let Ok(mut guard) = writer.lock() {
                if let Some(writer) = guard.as_mut() {
                    for sample in data {
                        let value = f32::from_sample(*sample).clamp(-1.0, 1.0);
                        sum_squares += value * value;
                        count += 1;
                        let value = (value * i16::MAX as f32) as i16;
                        let _ = writer.write_sample(value);
                    }
                }
            }

            if count > 0 {
                let rms = (sum_squares / count as f32).sqrt();
                level_callback((rms * 4.0).clamp(0.0, 1.0));
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}
