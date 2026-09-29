use std::{
    io::Cursor,
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, Sample, SampleFormat, SizedSample, StreamConfig,
};
use hound::{SampleFormat as WavSampleFormat, WavSpec, WavWriter};

use crate::{
    i18n::{self, tr, trf},
    AppError, AppResult,
};

/// Speech models are trained on 16 kHz mono; sending that instead of the
/// device's native 48 kHz stereo cuts the upload ~6x.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

pub type LevelCallback = Arc<dyn Fn(f32) + Send + Sync + 'static>;

/// Owns the audio thread. `cpal::Stream` is not `Send` on every platform, so
/// it lives and dies on its own thread and we talk to it over a channel.
pub struct RecordingSession {
    stop_tx: Sender<()>,
    handle: JoinHandle<Captured>,
    started_at: Instant,
}

struct Captured {
    samples: Vec<i16>,
    peak: f32,
}

pub struct StoppedRecording {
    pub wav: Vec<u8>,
    pub duration_seconds: f64,
    pub peak: f32,
}

impl RecordingSession {
    pub fn elapsed_seconds(&self) -> f64 {
        self.started_at.elapsed().as_secs_f64()
    }

    pub fn stop(self) -> AppResult<StoppedRecording> {
        let _ = self.stop_tx.send(());
        let captured = self
            .handle
            .join()
            .map_err(|_| AppError::Audio(tr(&i18n::AUDIO_THREAD_EXITED).to_string()))?;
        let duration_seconds = captured.samples.len() as f64 / TARGET_SAMPLE_RATE as f64;
        Ok(StoppedRecording {
            wav: encode_wav(&captured.samples)?,
            duration_seconds,
            peak: captured.peak,
        })
    }
}

pub fn list_input_devices() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|devices| devices.filter_map(|device| device.name().ok()).collect())
        .unwrap_or_default()
}

pub fn start(device_name: &str, level_callback: LevelCallback) -> AppResult<RecordingSession> {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (ready_tx, ready_rx) = mpsc::channel::<AppResult<()>>();
    let device_name = device_name.trim().to_string();

    let handle = std::thread::Builder::new()
        .name("audio-capture".to_string())
        .spawn(move || run_capture(&device_name, level_callback, ready_tx, stop_rx))
        .map_err(|error| AppError::Audio(trf(&i18n::AUDIO_THREAD_SPAWN, &[&error])))?;

    match ready_rx.recv() {
        Ok(Ok(())) => Ok(RecordingSession {
            stop_tx,
            handle,
            started_at: Instant::now(),
        }),
        Ok(Err(error)) => {
            let _ = handle.join();
            Err(error)
        }
        Err(_) => {
            let _ = handle.join();
            Err(AppError::Audio(tr(&i18n::AUDIO_START_FAILED).to_string()))
        }
    }
}

fn run_capture(
    device_name: &str,
    level_callback: LevelCallback,
    ready_tx: Sender<AppResult<()>>,
    stop_rx: Receiver<()>,
) -> Captured {
    let shared = Arc::new(Mutex::new(Captured {
        samples: Vec::with_capacity(TARGET_SAMPLE_RATE as usize * 60),
        peak: 0.0,
    }));

    let stream = match open_stream(device_name, shared.clone(), level_callback) {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready_tx.send(Err(error));
            return take(&shared);
        }
    };
    if let Err(error) = stream.play() {
        let _ = ready_tx.send(Err(error.into()));
        return take(&shared);
    }
    let _ = ready_tx.send(Ok(()));

    let _ = stop_rx.recv();
    drop(stream);
    take(&shared)
}

fn take(shared: &Arc<Mutex<Captured>>) -> Captured {
    let mut guard = shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    Captured {
        samples: std::mem::take(&mut guard.samples),
        peak: guard.peak,
    }
}

fn open_stream(
    device_name: &str,
    shared: Arc<Mutex<Captured>>,
    level_callback: LevelCallback,
) -> AppResult<cpal::Stream> {
    let host = cpal::default_host();
    let device = if device_name.is_empty() {
        None
    } else {
        host.input_devices()
            .ok()
            .and_then(|mut devices| devices.find(|d| d.name().ok().as_deref() == Some(device_name)))
    };
    let device = match device.or_else(|| host.default_input_device()) {
        Some(device) => device,
        None => {
            return Err(AppError::Audio(
                tr(&i18n::NO_MIC).to_string(),
            ))
        }
    };

    let supported_config = device.default_input_config()?;
    let sample_format = supported_config.sample_format();
    let config: StreamConfig = supported_config.into();

    macro_rules! build {
        ($t:ty) => {
            build_stream::<$t>(&device, &config, shared, level_callback)
        };
    }

    match sample_format {
        SampleFormat::I8 => build!(i8),
        SampleFormat::I16 => build!(i16),
        SampleFormat::I32 => build!(i32),
        SampleFormat::I64 => build!(i64),
        SampleFormat::U8 => build!(u8),
        SampleFormat::U16 => build!(u16),
        SampleFormat::U32 => build!(u32),
        SampleFormat::U64 => build!(u64),
        SampleFormat::F32 => build!(f32),
        SampleFormat::F64 => build!(f64),
        sample => Err(AppError::Audio(trf(
            &i18n::UNSUPPORTED_FORMAT,
            &[&format!("{sample:?}")],
        ))),
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    shared: Arc<Mutex<Captured>>,
    level_callback: LevelCallback,
) -> AppResult<cpal::Stream>
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels.max(1) as usize;
    let mut resampler = Resampler::new(config.sample_rate.0, TARGET_SAMPLE_RATE);
    let mut out = Vec::with_capacity(4096);

    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            let mut sum_squares = 0.0f32;
            let mut frames = 0usize;
            out.clear();

            for frame in data.chunks(channels) {
                let mono = frame.iter().map(|s| f32::from_sample(*s)).sum::<f32>()
                    / frame.len() as f32;
                let mono = mono.clamp(-1.0, 1.0);
                sum_squares += mono * mono;
                frames += 1;
                resampler.push(mono, &mut out);
            }

            if frames == 0 {
                return;
            }
            let rms = (sum_squares / frames as f32).sqrt();
            if let Ok(mut guard) = shared.lock() {
                guard
                    .samples
                    .extend(out.iter().map(|v| (v * i16::MAX as f32) as i16));
                guard.peak = guard.peak.max(rms);
            }
            level_callback((rms * 5.0).clamp(0.0, 1.0));
        },
        |error| eprintln!("audio input stream error: {error}"),
        None,
    )?;

    Ok(stream)
}

/// Linear-interpolating resampler with a one-pole low-pass in front to keep
/// aliasing out of the speech band when downsampling.
struct Resampler {
    step: f64,
    next_t: f64,
    index: u64,
    prev: f32,
    lp_state: f32,
    lp_alpha: f32,
}

impl Resampler {
    fn new(in_rate: u32, out_rate: u32) -> Self {
        let in_rate = in_rate.max(1) as f32;
        let cutoff = (out_rate as f32 * 0.45).min(in_rate * 0.45);
        let lp_alpha = if in_rate as u32 > out_rate {
            1.0 - (-2.0 * std::f32::consts::PI * cutoff / in_rate).exp()
        } else {
            1.0
        };
        Self {
            step: in_rate as f64 / out_rate as f64,
            next_t: 0.0,
            index: 0,
            prev: 0.0,
            lp_state: 0.0,
            lp_alpha,
        }
    }

    fn push(&mut self, sample: f32, out: &mut Vec<f32>) {
        self.lp_state += self.lp_alpha * (sample - self.lp_state);
        let x = self.lp_state;
        let n = self.index as f64;
        while self.next_t <= n {
            let frac = (self.next_t - (n - 1.0)) as f32;
            let value = if self.index == 0 {
                x
            } else {
                self.prev + (x - self.prev) * frac
            };
            out.push(value);
            self.next_t += self.step;
        }
        self.prev = x;
        self.index += 1;
    }
}

fn encode_wav(samples: &[i16]) -> AppResult<Vec<u8>> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: WavSampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::with_capacity(samples.len() * 2 + 44));
    {
        let mut writer = WavWriter::new(&mut cursor, spec)?;
        let mut sample_writer = writer.get_i16_writer(samples.len() as u32);
        for sample in samples {
            sample_writer.write_sample(*sample);
        }
        sample_writer.flush()?;
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampler_produces_expected_length() {
        let mut resampler = Resampler::new(48_000, 16_000);
        let mut out = Vec::new();
        for i in 0..48_000 {
            resampler.push(((i as f32) * 0.01).sin() * 0.5, &mut out);
        }
        assert!((out.len() as i64 - 16_000).abs() <= 1, "len {}", out.len());
    }

    #[test]
    fn encodes_valid_wav_header() {
        let wav = encode_wav(&[0, 1, -1, 100]).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(wav.len(), 44 + 8);
    }
}
