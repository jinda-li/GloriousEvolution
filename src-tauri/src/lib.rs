mod api;
mod audio;
mod history;
mod i18n;
mod optimizer;
mod paste;
mod settings;
mod stt;
mod text_filter;

use std::{
    str::FromStr,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};

use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalPosition, State, WindowEvent,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_opener::OpenerExt;
use thiserror::Error;

use i18n::{tr, trf};
use settings::RecordMode;

type AppResult<T> = Result<T, AppError>;

/// Recordings are capped so a forgotten session cannot run up an API bill.
const MAX_RECORDING_SECONDS: u64 = 10 * 60;
const MIN_RECORDING_SECONDS: f64 = 0.4;
/// RMS below this across the whole take means the mic heard nothing.
const SILENCE_PEAK: f32 = 0.004;
const AUTOSTART_FLAG: &str = "--minimized";
const TRAY_ID: &str = "sayso";

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
    Clipboard(#[from] arboard::Error),
    #[error(transparent)]
    Hound(#[from] hound::Error),
    #[error("{}", mic_unavailable(.0))]
    CpalDefaultConfig(#[from] cpal::DefaultStreamConfigError),
    #[error("{}", mic_unavailable(.0))]
    CpalBuildStream(#[from] cpal::BuildStreamError),
    #[error("{}", mic_unavailable(.0))]
    CpalPlayStream(#[from] cpal::PlayStreamError),
}

fn mic_unavailable(error: &dyn std::fmt::Display) -> String {
    trf(&i18n::MIC_UNAVAILABLE, &[&error])
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
    processing: AtomicBool,
    session_id: AtomicU64,
    registered_shortcut: Mutex<Option<Shortcut>>,
    cancel_shortcut_active: AtomicBool,
    last_shortcut_toggle: Mutex<Instant>,
    shortcut_capture_mode: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            recording: Mutex::new(None),
            processing: AtomicBool::new(false),
            session_id: AtomicU64::new(0),
            registered_shortcut: Mutex::new(None),
            cancel_shortcut_active: AtomicBool::new(false),
            last_shortcut_toggle: Mutex::new(Instant::now() - Duration::from_secs(1)),
            shortcut_capture_mode: AtomicBool::new(false),
        }
    }
}

fn cancel_shortcut() -> Shortcut {
    Shortcut::new(None, Code::Escape)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessResult {
    raw_text: String,
    optimized_text: String,
    duration_seconds: f64,
    delivery: &'static str,
    warning: Option<String>,
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
    optimize_ms: u128,
    paste_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextPreviewEvent {
    text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    version: &'static str,
    default_system_prompt: String,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

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
    // Validate the shortcut before persisting so a typo cannot brick the hotkey.
    Shortcut::from_str(new_settings.shortcut.trim()).map_err(|error| {
        AppError::Config(trf(&i18n::INVALID_SHORTCUT, &[&new_settings.shortcut, &error]))
    })?;
    settings::save(&app, &new_settings)?;
    register_shortcut_from_settings(&app, state.inner())?;
    apply_autostart(&app, new_settings.launch_at_login);
    Ok(())
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        default_system_prompt: settings::default_system_prompt(),
    }
}

/// Called by the main window whenever its resolved interface language changes.
#[tauri::command]
fn set_locale(app: AppHandle, locale: String) {
    i18n::set(i18n::parse(&locale).unwrap_or(i18n::Locale::En));
    refresh_tray(&app);
}

#[tauri::command]
async fn test_connection(new_settings: settings::AppSettings) -> AppResult<api::KeyStatus> {
    api::check_key(&new_settings).await
}

#[tauri::command]
fn list_input_devices() -> Vec<String> {
    audio::list_input_devices()
}

#[tauri::command]
fn get_history(app: AppHandle) -> AppResult<history::HistoryStore> {
    history::load(&app)
}

#[tauri::command]
fn delete_history_entry(app: AppHandle, id: String) -> AppResult<history::HistoryStore> {
    history::delete(&app, &id)
}

#[tauri::command]
fn clear_history(app: AppHandle) -> AppResult<history::HistoryStore> {
    history::clear(&app)
}

#[tauri::command]
fn open_external(app: AppHandle, url: String) -> AppResult<()> {
    if !url.starts_with("https://") {
        return Err(AppError::Config(tr(&i18n::HTTPS_ONLY).to_string()));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| AppError::Config(error.to_string()))
}

#[tauri::command]
fn copy_text(text: String) -> AppResult<()> {
    paste::write_clipboard(&text)
}

#[tauri::command]
fn set_shortcut_capture_mode(state: State<'_, AppState>, capturing: bool) {
    state.shortcut_capture_mode.store(capturing, Ordering::SeqCst);
}

#[tauri::command]
fn start_recording(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    start_recording_inner(&app, state.inner())
}

#[tauri::command]
async fn stop_and_process(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    stop_and_process_inner(&app, state.inner()).await
}

#[tauri::command]
fn cancel_recording(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    cancel_recording_inner(&app, state.inner())
}

#[tauri::command]
async fn toggle_recording(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    toggle_recording_inner(&app, state.inner()).await
}

#[tauri::command]
fn hide_recorder(app: AppHandle) -> AppResult<()> {
    hide_recorder_window(&app)
}

// ---------------------------------------------------------------------------
// Recording pipeline
// ---------------------------------------------------------------------------

fn is_recording(state: &AppState) -> bool {
    state
        .recording
        .lock()
        .map(|recording| recording.is_some())
        .unwrap_or(false)
}

fn start_recording_inner(app: &AppHandle, state: &AppState) -> AppResult<()> {
    if state.processing.load(Ordering::SeqCst) {
        return Ok(());
    }

    let settings = settings::load(app)?;
    if settings.api_key.trim().is_empty() {
        show_main_window(app);
        let message = tr(&i18n::NEED_KEY_IN_SETTINGS).to_string();
        emit_recording(app, "error", &message);
        return Err(AppError::Config(message));
    }

    let mut recording = state
        .recording
        .lock()
        .map_err(|_| AppError::Audio(tr(&i18n::LOCK_RECORDING).to_string()))?;
    if recording.is_some() {
        return Ok(());
    }

    let app_for_level = app.clone();
    let last_emit = Mutex::new(Instant::now() - Duration::from_millis(100));
    let session = audio::start(
        &settings.input_device,
        std::sync::Arc::new(move |level| {
            if let Ok(mut last_emit) = last_emit.lock() {
                if last_emit.elapsed() >= Duration::from_millis(50) {
                    *last_emit = Instant::now();
                    let _ = app_for_level.emit("audio-level", AudioLevelEvent { level });
                }
            }
        }),
    )?;
    *recording = Some(session);
    drop(recording);

    let session_id = state.session_id.fetch_add(1, Ordering::SeqCst) + 1;
    set_cancel_shortcut(app, state, true);
    show_recorder_window(app)?;
    let hint = match settings.record_mode {
        RecordMode::Toggle => tr(&i18n::LISTENING_TOGGLE),
        RecordMode::Hold => tr(&i18n::LISTENING_HOLD),
    };
    emit_recording(app, "recording", hint);
    spawn_recording_watchdog(app.clone(), session_id);
    Ok(())
}

fn spawn_recording_watchdog(app: AppHandle, session_id: u64) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(MAX_RECORDING_SECONDS)).await;
        let state = app.state::<AppState>();
        if state.session_id.load(Ordering::SeqCst) == session_id && is_recording(state.inner()) {
            let _ = stop_and_process_inner(&app, state.inner()).await;
        }
    });
}

async fn stop_and_process_inner(app: &AppHandle, state: &AppState) -> AppResult<()> {
    if state.processing.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let session = match state.recording.lock() {
        Ok(mut recording) => recording.take(),
        Err(_) => None,
    };
    let Some(session) = session else {
        state.processing.store(false, Ordering::SeqCst);
        return Ok(());
    };
    set_cancel_shortcut(app, state, false);

    let result = process_session(app, session).await;
    state.processing.store(false, Ordering::SeqCst);

    if let Err(error) = &result {
        emit_recording(app, "error", &error.to_string());
    }
    result.map(|_| ())
}

/// Returns `None` when the take was too short to be worth transcribing.
async fn process_session(
    app: &AppHandle,
    session: audio::RecordingSession,
) -> AppResult<Option<ProcessResult>> {
    emit_recording(app, "processing", tr(&i18n::RECOGNIZING));
    emit_progress(app, "stopping", tr(&i18n::PHASE_PREPARE), 0.1);
    let total_start = Instant::now();
    let too_short = session.elapsed_seconds() < MIN_RECORDING_SECONDS;
    let stopped = tauri::async_runtime::spawn_blocking(move || session.stop())
        .await
        .map_err(|error| AppError::Audio(error.to_string()))??;

    if too_short || stopped.duration_seconds < MIN_RECORDING_SECONDS {
        hide_recorder_window(app)?;
        emit_recording(app, "idle", tr(&i18n::TOO_SHORT));
        return Ok(None);
    }
    if stopped.peak < SILENCE_PEAK {
        return Err(AppError::Audio(tr(&i18n::NO_SOUND).to_string()));
    }

    let settings = settings::load(app)?;

    emit_progress(app, "stt", tr(&i18n::PHASE_STT), 0.3);
    let stt_start = Instant::now();
    let raw_text = stt::transcribe(stopped.wav, &settings).await?;
    let stt_ms = stt_start.elapsed().as_millis();

    let human_text = text_filter::clean_human_speech_text(&raw_text, settings.language.trim())?;

    let optimize_start = Instant::now();
    let mut warning = None;
    let mut polished = false;
    let final_text = if settings.polish_enabled && !settings.llm_model.trim().is_empty() {
        emit_progress(app, "optimize", tr(&i18n::PHASE_POLISH), 0.65);
        match optimizer::optimize(&human_text, &settings).await {
            Ok(text) if !text.trim().is_empty() => {
                polished = true;
                text
            }
            Ok(_) => human_text.clone(),
            // Never lose a dictation because the polish step failed.
            Err(error) => {
                warning = Some(trf(&i18n::POLISH_FAILED, &[&error]));
                human_text.clone()
            }
        }
    } else {
        human_text.clone()
    };
    let optimize_ms = optimize_start.elapsed().as_millis();

    emit_progress(app, "paste", tr(&i18n::PHASE_OUTPUT), 0.92);
    let paste_start = Instant::now();
    let delivery = {
        let settings = settings.clone();
        let text = final_text.clone();
        tauri::async_runtime::spawn_blocking(move || paste::deliver_text(&settings, &text))
            .await
            .map_err(|error| AppError::Audio(error.to_string()))??
    };
    let paste_ms = paste_start.elapsed().as_millis();

    let delivery_label = match delivery {
        paste::TextDelivery::Inserted => {
            hide_recorder_window(app)?;
            emit_recording(app, "done", tr(&i18n::INSERTED));
            "inserted"
        }
        paste::TextDelivery::NeedsCopy => {
            emit_text_preview(app, &final_text);
            emit_recording(app, "preview", tr(&i18n::NO_TEXT_FIELD));
            "needsCopy"
        }
    };
    emit_progress(app, "done", tr(&i18n::PHASE_DONE), 1.0);

    if let Ok(store) = history::record(
        app,
        settings.history_enabled,
        &human_text,
        &final_text,
        stopped.duration_seconds,
        polished,
    ) {
        let _ = app.emit("history-updated", &store);
    }

    let result = ProcessResult {
        raw_text: human_text,
        optimized_text: final_text,
        duration_seconds: stopped.duration_seconds,
        delivery: delivery_label,
        warning,
    };
    let _ = app.emit("process-result", &result);
    let _ = app.emit(
        "latency-metrics",
        LatencyMetricsEvent {
            total_ms: total_start.elapsed().as_millis(),
            stt_ms,
            optimize_ms,
            paste_ms,
        },
    );
    Ok(Some(result))
}

fn cancel_recording_inner(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let session = state
        .recording
        .lock()
        .map_err(|_| AppError::Audio(tr(&i18n::LOCK_RECORDING).to_string()))?
        .take();
    set_cancel_shortcut(app, state, false);

    if let Some(session) = session {
        std::thread::spawn(move || {
            let _ = session.stop();
        });
    }

    hide_recorder_window(app)?;
    emit_recording(app, "idle", tr(&i18n::CANCELLED));
    Ok(())
}

async fn toggle_recording_inner(app: &AppHandle, state: &AppState) -> AppResult<()> {
    if is_recording(state) {
        stop_and_process_inner(app, state).await
    } else {
        start_recording_inner(app, state)
    }
}

// ---------------------------------------------------------------------------
// Shortcuts
// ---------------------------------------------------------------------------

fn handle_global_shortcut(app: &AppHandle, shortcut: &Shortcut, pressed: bool) {
    let state = app.state::<AppState>();
    if state.shortcut_capture_mode.load(Ordering::SeqCst) {
        return;
    }

    if *shortcut == cancel_shortcut() {
        if pressed && state.cancel_shortcut_active.load(Ordering::SeqCst) {
            let _ = cancel_recording_inner(app, state.inner());
        }
        return;
    }

    let mode = settings::load(app)
        .map(|settings| settings.record_mode)
        .unwrap_or_default();

    let action = match (mode, pressed) {
        (RecordMode::Toggle, true) => {
            if let Ok(mut last_toggle) = state.last_shortcut_toggle.lock() {
                if last_toggle.elapsed() < Duration::from_millis(300) {
                    return;
                }
                *last_toggle = Instant::now();
            }
            ShortcutAction::Toggle
        }
        (RecordMode::Toggle, false) => return,
        (RecordMode::Hold, true) => ShortcutAction::Start,
        (RecordMode::Hold, false) => ShortcutAction::Stop,
    };

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let result = match action {
            ShortcutAction::Toggle => toggle_recording_inner(&app, state.inner()).await,
            ShortcutAction::Start => {
                if is_recording(state.inner()) {
                    Ok(())
                } else {
                    start_recording_inner(&app, state.inner())
                }
            }
            ShortcutAction::Stop => stop_and_process_inner(&app, state.inner()).await,
        };
        if let Err(error) = result {
            emit_recording(&app, "error", &error.to_string());
        }
    });
}

#[derive(Clone, Copy)]
enum ShortcutAction {
    Toggle,
    Start,
    Stop,
}

fn set_cancel_shortcut(app: &AppHandle, state: &AppState, active: bool) {
    let was_active = state.cancel_shortcut_active.swap(active, Ordering::SeqCst);
    if was_active == active {
        return;
    }
    let shortcuts = app.global_shortcut();
    if active {
        // Another app may own Esc globally; recording still works without it.
        let _ = shortcuts.register(cancel_shortcut());
    } else {
        let _ = shortcuts.unregister(cancel_shortcut());
    }
}

fn register_shortcut_from_settings(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let settings = settings::load(app)?;
    let shortcut = Shortcut::from_str(settings.shortcut.trim()).map_err(|error| {
        AppError::Config(trf(&i18n::INVALID_SHORTCUT, &[&settings.shortcut, &error]))
    })?;

    let mut registered = state
        .registered_shortcut
        .lock()
        .map_err(|_| AppError::Config(tr(&i18n::LOCK_SHORTCUT).to_string()))?;

    if registered.as_ref() == Some(&shortcut) {
        return Ok(());
    }

    if let Some(previous) = registered.take() {
        let _ = app.global_shortcut().unregister(previous);
    }

    app.global_shortcut().register(shortcut).map_err(|error| {
        AppError::Config(trf(&i18n::SHORTCUT_TAKEN, &[&settings.shortcut, &error]))
    })?;
    *registered = Some(shortcut);
    Ok(())
}

fn apply_autostart(app: &AppHandle, enabled: bool) {
    let autolaunch = app.autolaunch();
    let current = autolaunch.is_enabled().unwrap_or(false);
    if enabled && !current {
        let _ = autolaunch.enable();
    } else if !enabled && current {
        let _ = autolaunch.disable();
    }
}

// ---------------------------------------------------------------------------
// App setup
// ---------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main_window(app);
        }))
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![AUTOSTART_FLAG]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    handle_global_shortcut(app, shortcut, event.state == ShortcutState::Pressed);
                })
                .build(),
        )
        .setup(|app| {
            settings::migrate_legacy_data(app.handle());
            let settings = settings::load(app.handle()).unwrap_or_default();
            i18n::apply_setting(&settings.ui_language);
            setup_tray(app.handle())?;
            let state = app.state::<AppState>();
            if let Err(error) = register_shortcut_from_settings(app.handle(), state.inner()) {
                eprintln!("{error}");
            }

            let started_hidden = std::env::args().any(|arg| arg == AUTOSTART_FLAG);
            if !started_hidden || settings.api_key.trim().is_empty() {
                show_main_window(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the main window keeps the app alive in the tray.
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            save_settings,
            app_info,
            set_locale,
            test_connection,
            list_input_devices,
            get_history,
            delete_history_entry,
            clear_history,
            open_external,
            copy_text,
            set_shortcut_capture_mode,
            start_recording,
            stop_and_process,
            cancel_recording,
            toggle_recording,
            hide_recorder
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sayso");
}

fn build_tray_menu(app: &AppHandle) -> AppResult<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", tr(&i18n::TRAY_SHOW), true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", tr(&i18n::TRAY_TOGGLE), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", tr(&i18n::TRAY_QUIT), true, None::<&str>)?;
    Ok(Menu::with_items(app, &[&show, &toggle, &separator, &quit])?)
}

fn refresh_tray(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = build_tray_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
    let _ = tray.set_tooltip(Some(tr(&i18n::TRAY_TOOLTIP)));
}

fn setup_tray(app: &AppHandle) -> AppResult<()> {
    let menu = build_tray_menu(app)?;
    let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tr(&i18n::TRAY_TOOLTIP))
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main_window(app),
            "toggle" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = app.state::<AppState>();
                    if let Err(error) = toggle_recording_inner(&app, state.inner()).await {
                        emit_recording(&app, "error", &error.to_string());
                    }
                });
            }
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

// ---------------------------------------------------------------------------
// Windows & events
// ---------------------------------------------------------------------------

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn show_recorder_window(app: &AppHandle) -> AppResult<()> {
    let Some(window) = app.get_webview_window("recorder") else {
        return Ok(());
    };

    // Follow the cursor so the pill appears on the monitor the user is working on.
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|point| app.monitor_from_point(point.x, point.y).ok().flatten())
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        let win_size = window.outer_size()?;
        let margin = (24.0 * monitor.scale_factor()) as i32;
        let x = area.position.x + (area.size.width.saturating_sub(win_size.width) / 2) as i32;
        let y = area.position.y + area.size.height.saturating_sub(win_size.height) as i32 - margin;
        window.set_position(PhysicalPosition { x, y })?;
    }
    window.show()?;
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

fn emit_text_preview(app: &AppHandle, text: &str) {
    let _ = app.emit(
        "text-preview",
        TextPreviewEvent {
            text: text.to_string(),
        },
    );
}
