mod audio;
mod optimizer;
mod paste;
mod settings;
mod stt;
mod text_filter;

use std::{
    str::FromStr,
    sync::Mutex,
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, State,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use thiserror::Error;

type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Audio(String),
    #[error("{0}")]
    Api(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error(transparent)]
    GlobalShortcut(#[from] tauri_plugin_global_shortcut::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    Clipboard(#[from] arboard::Error),
    #[error(transparent)]
    Hound(#[from] hound::Error),
    #[error(transparent)]
    CpalDefaultConfig(#[from] cpal::DefaultStreamConfigError),
    #[error(transparent)]
    CpalBuildStream(#[from] cpal::BuildStreamError),
    #[error(transparent)]
    CpalPlayStream(#[from] cpal::PlayStreamError),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct AppState {
    recording: Mutex<Option<audio::RecordingSession>>,
    registered_shortcut: Mutex<Option<Shortcut>>,
    last_shortcut_toggle: Mutex<Instant>,
    shortcut_capture_mode: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            recording: Mutex::new(None),
            registered_shortcut: Mutex::new(None),
            last_shortcut_toggle: Mutex::new(Instant::now() - Duration::from_secs(1)),
            shortcut_capture_mode: Mutex::new(false),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessResult {
    raw_text: String,
    optimized_text: String,
    duration_seconds: f64,
    delivery: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordingEvent {
    status: &'static str,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioLevelEvent {
    level: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessingProgressEvent {
    phase: &'static str,
    message: String,
    progress: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LatencyMetricsEvent {
    total_ms: u128,
    stt_ms: u128,
    filter_ms: u128,
    optimize_ms: u128,
    paste_ms: u128,
    bottleneck: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextPreviewEvent {
    text: String,
}

#[tauri::command]
fn load_settings(app: AppHandle) -> AppResult<settings::AppSettings> {
    settings::load(&app)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    new_settings: settings::AppSettings,
) -> AppResult<()> {
    settings::save(&app, &new_settings)?;
    register_shortcut_from_settings(&app, state.inner())?;
    Ok(())
}

#[tauri::command]
fn copy_text(text: String) -> AppResult<()> {
    paste::write_clipboard(&text)
}

#[tauri::command]
fn set_shortcut_capture_mode(state: State<'_, AppState>, capturing: bool) -> AppResult<()> {
    let mut mode = state
        .shortcut_capture_mode
        .lock()
        .map_err(|_| AppError::Config("快捷键录入状态锁定失败。".to_string()))?;
    *mode = capturing;
    Ok(())
}

#[tauri::command]
fn start_recording(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    start_recording_inner(&app, state.inner())
}

fn start_recording_inner(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let mut recording = state
        .recording
        .lock()
        .map_err(|_| AppError::Audio("录音状态锁定失败。".to_string()))?;

    if recording.is_some() {
        return Ok(());
    }

    let app_for_level = app.clone();
    let last_emit = std::sync::Arc::new(Mutex::new(Instant::now() - Duration::from_millis(100)));
    let last_emit_for_callback = last_emit.clone();
    let session = audio::start(std::sync::Arc::new(move |level| {
        if let Ok(mut last_emit) = last_emit_for_callback.lock() {
            if last_emit.elapsed() >= Duration::from_millis(80) {
                *last_emit = Instant::now();
                let _ = app_for_level.emit("audio-level", AudioLevelEvent { level });
            }
        }
    }))?;
    *recording = Some(session);
    show_recorder_window(app)?;
    emit_recording(app, "recording", "正在录音，再次按快捷键结束。");
    Ok(())
}

#[tauri::command]
async fn stop_and_process(app: AppHandle, state: State<'_, AppState>) -> AppResult<ProcessResult> {
    stop_and_process_inner(&app, state.inner()).await
}

async fn stop_and_process_inner(app: &AppHandle, state: &AppState) -> AppResult<ProcessResult> {
    let session = {
        let mut recording = state
            .recording
            .lock()
            .map_err(|_| AppError::Audio("录音状态锁定失败。".to_string()))?;
        recording.take()
    };

    let Some(session) = session else {
        return Err(AppError::Audio("当前没有正在进行的录音。".to_string()));
    };

    emit_recording(app, "processing", "正在处理语音。");
    emit_progress(app, "stopping", "保存录音", 0.12);
    let total_start = Instant::now();
    let stopped = session.stop()?;
    let settings = settings::load(app)?;
    if let Some(message) = missing_required_config_message(&settings) {
        let _ = std::fs::remove_file(stopped.path);
        emit_recording(app, "error", &message);
        return Err(AppError::Config(message));
    }

    emit_progress(app, "stt", "语音识别", 0.28);
    let stt_start = Instant::now();
    let raw_text = stt::transcribe(&stopped.path, &settings.elevenlabs_api_key).await?;
    let stt_ms = stt_start.elapsed().as_millis();

    emit_progress(app, "filter", "清洗文本", 0.58);
    let filter_start = Instant::now();
    let human_text = text_filter::clean_human_speech_text(&raw_text)?;
    let filter_ms = filter_start.elapsed().as_millis();

    emit_progress(app, "optimize", "文本优化", 0.72);
    let optimize_start = Instant::now();
    let optimized_text = optimizer::optimize(&human_text, &settings).await?;
    let optimize_ms = optimize_start.elapsed().as_millis();

    emit_progress(app, "paste", "输出文本", 0.92);
    let paste_start = Instant::now();
    let delivery = paste::deliver_text(&settings, &optimized_text)?;
    let paste_ms = paste_start.elapsed().as_millis();

    let delivery_label = match delivery {
        paste::TextDelivery::Inserted => {
            hide_recorder_window(app)?;
            emit_recording(app, "done", "文本已输入。");
            "inserted"
        }
        paste::TextDelivery::NeedsCopy => {
            emit_text_preview(app, &optimized_text);
            emit_recording(app, "preview", "未找到输入焦点，请复制。");
            "needsCopy"
        }
    };
    emit_progress(app, "done", "完成", 1.0);

    let _ = std::fs::remove_file(stopped.path);
    let result = ProcessResult {
        raw_text: human_text,
        optimized_text,
        duration_seconds: stopped.duration_seconds,
        delivery: delivery_label,
    };
    let _ = app.emit("process-result", &result);
    emit_latency_metrics(
        app,
        LatencyMetricsEvent {
            total_ms: total_start.elapsed().as_millis(),
            stt_ms,
            filter_ms,
            optimize_ms,
            paste_ms,
            bottleneck: bottleneck(stt_ms, filter_ms, optimize_ms, paste_ms),
        },
    );
    Ok(result)
}

fn missing_required_config_message(settings: &settings::AppSettings) -> Option<String> {
    if settings.elevenlabs_api_key.trim().is_empty() {
        return Some("请先在设置中填写 ElevenLabs API Key。".to_string());
    }

    if settings.optimizer_api_key.trim().is_empty() {
        return Some("请先在设置中填写文本模型 API Key。".to_string());
    }

    if settings.optimizer_base_url.trim().is_empty() {
        return Some("请先在设置中填写文本模型 API Base URL。".to_string());
    }

    if settings.optimizer_model.trim().is_empty() {
        return Some("请先在设置中填写文本模型名称。".to_string());
    }

    None
}

#[tauri::command]
fn cancel_recording(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    cancel_recording_inner(&app, state.inner())
}

fn cancel_recording_inner(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let session = {
        let mut recording = state
            .recording
            .lock()
            .map_err(|_| AppError::Audio("录音状态锁定失败。".to_string()))?;
        recording.take()
    };

    if let Some(session) = session {
        let stopped = session.stop()?;
        let _ = std::fs::remove_file(stopped.path);
    }

    hide_recorder_window(app)?;
    emit_recording(app, "idle", "录音已取消。");
    Ok(())
}

#[tauri::command]
async fn toggle_recording(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    toggle_recording_inner(&app, state.inner()).await
}

async fn toggle_recording_inner(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let is_recording = state
        .recording
        .lock()
        .map_err(|_| AppError::Audio("录音状态锁定失败。".to_string()))?
        .is_some();

    if is_recording {
        let _ = stop_and_process_inner(app, state).await?;
    } else {
        start_recording_inner(app, state)?;
    }

    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }

                    let state = app.state::<AppState>();
                    if state
                        .shortcut_capture_mode
                        .lock()
                        .map(|mode| *mode)
                        .unwrap_or(false)
                    {
                        return;
                    }

                    if let Ok(mut last_toggle) = state.last_shortcut_toggle.lock() {
                        if last_toggle.elapsed() < Duration::from_millis(400) {
                            return;
                        }
                        *last_toggle = Instant::now();
                    }

                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let state = app.state::<AppState>();
                        if let Err(error) = toggle_recording_inner(&app, state.inner()).await {
                            emit_recording(&app, "error", &error.to_string());
                        }
                    });
                })
                .build(),
        )
        .setup(|app| {
            setup_tray(app.handle())?;
            let state = app.state::<AppState>();
            register_shortcut_from_settings(app.handle(), state.inner())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            copy_text,
            set_shortcut_capture_mode,
            start_recording,
            stop_and_process,
            cancel_recording,
            toggle_recording
        ])
        .run(tauri::generate_context!())
        .expect("error while running GloriousEvolution");
}

fn setup_tray(app: &AppHandle) -> AppResult<()> {
    let show = MenuItem::with_id(app, "show", "Show GloriousEvolution", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    TrayIconBuilder::with_id("glorious-evolution")
        .tooltip("GloriousEvolution")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

fn register_shortcut_from_settings(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let settings = settings::load(app)?;
    let shortcut = Shortcut::from_str(settings.shortcut.trim()).map_err(|error| {
        AppError::Config(format!("无效的快捷键 \"{}\": {}", settings.shortcut, error))
    })?;

    let mut registered = state
        .registered_shortcut
        .lock()
        .map_err(|_| AppError::Config("快捷键状态锁定失败。".to_string()))?;

    if registered.as_ref() == Some(&shortcut) {
        return Ok(());
    }

    if let Some(previous) = registered.take() {
        app.global_shortcut().unregister(previous)?;
    }

    app.global_shortcut().register(shortcut.clone())?;
    *registered = Some(shortcut);
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_recorder_window(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("recorder") {
        if let Some(monitor) = window.current_monitor()? {
            let size = monitor.size();
            let win_size = window.outer_size()?;
            let x = (size.width.saturating_sub(win_size.width) / 2) as i32;
            let y = (size.height.saturating_sub(win_size.height) as i32) - 92;
            window.set_position(PhysicalPosition { x, y })?;
        }
        window.show()?;
    }
    Ok(())
}

fn hide_recorder_window(app: &AppHandle) -> AppResult<()> {
    if let Some(window) = app.get_webview_window("recorder") {
        window.hide()?;
    }
    Ok(())
}

fn emit_recording(app: &AppHandle, status: &'static str, message: &str) {
    let _ = app.emit(
        "recording-status",
        RecordingEvent {
            status,
            message: message.to_string(),
        },
    );
}

fn emit_progress(app: &AppHandle, phase: &'static str, message: &str, progress: f32) {
    let _ = app.emit(
        "processing-progress",
        ProcessingProgressEvent {
            phase,
            message: message.to_string(),
            progress,
        },
    );
}

fn emit_latency_metrics(app: &AppHandle, metrics: LatencyMetricsEvent) {
    let _ = app.emit("latency-metrics", metrics);
}

fn emit_text_preview(app: &AppHandle, text: &str) {
    let _ = app.emit(
        "text-preview",
        TextPreviewEvent {
            text: text.to_string(),
        },
    );
}

fn bottleneck(stt_ms: u128, filter_ms: u128, optimize_ms: u128, paste_ms: u128) -> &'static str {
    let mut winner = ("语音识别", stt_ms);
    for candidate in [
        ("文本清洗", filter_ms),
        ("文本优化", optimize_ms),
        ("粘贴写入", paste_ms),
    ] {
        if candidate.1 > winner.1 {
            winner = candidate;
        }
    }
    winner.0
}
