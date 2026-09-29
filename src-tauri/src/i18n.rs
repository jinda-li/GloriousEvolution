//! Localized strings for text produced by the backend: errors, progress, tray.
//!
//! The UI language is chosen by the frontend and pushed here with `set_locale`;
//! before that happens (tray setup, autostart) the saved setting or the Windows
//! display language is used.

use std::fmt::Display;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    ZhCn,
    ZhTw,
    En,
    Ja,
    Ko,
    Es,
    Fr,
    De,
}

const ALL: [Locale; 8] = [
    Locale::ZhCn,
    Locale::ZhTw,
    Locale::En,
    Locale::Ja,
    Locale::Ko,
    Locale::Es,
    Locale::Fr,
    Locale::De,
];

static CURRENT: AtomicUsize = AtomicUsize::new(Locale::En as usize);

pub fn current() -> Locale {
    ALL.get(CURRENT.load(Ordering::Relaxed))
        .copied()
        .unwrap_or(Locale::En)
}

pub fn set(locale: Locale) {
    CURRENT.store(locale as usize, Ordering::Relaxed);
}

/// Parses a BCP 47 tag such as `zh-CN`, `zh-Hant`, `en-US` or `de`.
pub fn parse(tag: &str) -> Option<Locale> {
    let tag = tag.trim().to_ascii_lowercase().replace('_', "-");
    let primary = tag.split('-').next().unwrap_or_default();
    let locale = match primary {
        "zh" => {
            let traditional = tag
                .split('-')
                .skip(1)
                .any(|part| matches!(part, "tw" | "hk" | "mo" | "hant"));
            if traditional {
                Locale::ZhTw
            } else {
                Locale::ZhCn
            }
        }
        "en" => Locale::En,
        "ja" => Locale::Ja,
        "ko" => Locale::Ko,
        "es" => Locale::Es,
        "fr" => Locale::Fr,
        "de" => Locale::De,
        _ => return None,
    };
    Some(locale)
}

/// The Windows display language, falling back to English.
pub fn system() -> Locale {
    // LANGID: low 10 bits are the primary language, the rest the sublanguage.
    let langid = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() };
    let sublanguage = langid >> 10;
    match langid & 0x3ff {
        0x04 if matches!(sublanguage, 0x01 | 0x03 | 0x05) => Locale::ZhTw,
        0x04 => Locale::ZhCn,
        0x11 => Locale::Ja,
        0x12 => Locale::Ko,
        0x0a => Locale::Es,
        0x0c => Locale::Fr,
        0x07 => Locale::De,
        _ => Locale::En,
    }
}

/// Applies the saved `uiLanguage` setting; `auto` follows Windows.
pub fn apply_setting(ui_language: &str) {
    set(parse(ui_language).unwrap_or_else(system));
}

/// One message in every locale, ordered like [`Locale`].
pub struct Msg([&'static str; 8]);

pub fn tr(msg: &Msg) -> &'static str {
    msg.0[current() as usize]
}

/// Fills `{0}`, `{1}`, ... placeholders.
pub fn trf(msg: &Msg, args: &[&dyn Display]) -> String {
    let mut text = tr(msg).to_string();
    for (index, arg) in args.iter().enumerate() {
        text = text.replace(&format!("{{{index}}}"), &arg.to_string());
    }
    text
}

// ---------------------------------------------------------------------------
// Network and API
// ---------------------------------------------------------------------------

pub const PARSE_FAILED: Msg = Msg([
    "{0}返回了无法解析的数据：{1}",
    "{0}回傳了無法解析的資料：{1}",
    "{0} returned data that could not be read: {1}",
    "{0}から解析できないデータが返されました：{1}",
    "{0}에서 해석할 수 없는 데이터를 반환했습니다: {1}",
    "{0} devolvió datos que no se pudieron leer: {1}",
    "{0} a renvoyé des données illisibles : {1}",
    "{0} hat nicht lesbare Daten zurückgegeben: {1}",
]);

pub const NET_TIMEOUT: Msg = Msg([
    "网络请求超时，请检查网络后重试。",
    "網路請求逾時，請檢查網路後重試。",
    "The network request timed out. Check your connection and try again.",
    "ネットワーク要求がタイムアウトしました。接続を確認して再試行してください。",
    "네트워크 요청 시간이 초과되었습니다. 연결을 확인한 후 다시 시도하세요.",
    "La solicitud de red agotó el tiempo de espera. Revisa tu conexión e inténtalo de nuevo.",
    "La requête réseau a expiré. Vérifiez votre connexion et réessayez.",
    "Zeitüberschreitung der Netzwerkanfrage. Prüfe deine Verbindung und versuche es erneut.",
]);

pub const NET_CONNECT: Msg = Msg([
    "无法连接到服务器，请检查网络或代理设置。",
    "無法連線到伺服器，請檢查網路或代理設定。",
    "Could not reach the server. Check your network or proxy settings.",
    "サーバーに接続できません。ネットワークまたはプロキシ設定を確認してください。",
    "서버에 연결할 수 없습니다. 네트워크 또는 프록시 설정을 확인하세요.",
    "No se pudo conectar con el servidor. Revisa la red o la configuración del proxy.",
    "Impossible de joindre le serveur. Vérifiez votre réseau ou vos paramètres de proxy.",
    "Server nicht erreichbar. Prüfe dein Netzwerk oder die Proxy-Einstellungen.",
]);

pub const NET_FAILED: Msg = Msg([
    "网络请求失败：{0}",
    "網路請求失敗：{0}",
    "Network request failed: {0}",
    "ネットワーク要求に失敗しました：{0}",
    "네트워크 요청 실패: {0}",
    "La solicitud de red falló: {0}",
    "Échec de la requête réseau : {0}",
    "Netzwerkanfrage fehlgeschlagen: {0}",
]);

pub const HTTP_401: Msg = Msg([
    "API Key 无效或已被删除，请在设置中重新填写。",
    "API Key 無效或已被刪除，請在設定中重新填寫。",
    "The API key is invalid or was deleted. Enter it again in Settings.",
    "API キーが無効か削除されています。設定で入力し直してください。",
    "API 키가 유효하지 않거나 삭제되었습니다. 설정에서 다시 입력하세요.",
    "La clave de API no es válida o se eliminó. Vuelve a introducirla en Ajustes.",
    "La clé API est invalide ou a été supprimée. Saisissez-la à nouveau dans les Réglages.",
    "Der API-Schlüssel ist ungültig oder wurde gelöscht. Gib ihn in den Einstellungen erneut ein.",
]);

pub const HTTP_402: Msg = Msg([
    "OpenRouter 余额不足。语音识别要求账户余额至少 $0.50，请前往 openrouter.ai/settings/credits 充值。",
    "OpenRouter 餘額不足。語音辨識要求帳戶餘額至少 $0.50，請前往 openrouter.ai/settings/credits 儲值。",
    "Not enough OpenRouter credit. Speech recognition needs a balance of at least $0.50. Top up at openrouter.ai/settings/credits.",
    "OpenRouter の残高が不足しています。音声認識には $0.50 以上の残高が必要です。openrouter.ai/settings/credits でチャージしてください。",
    "OpenRouter 잔액이 부족합니다. 음성 인식에는 최소 $0.50의 잔액이 필요합니다. openrouter.ai/settings/credits 에서 충전하세요.",
    "Saldo de OpenRouter insuficiente. El reconocimiento de voz requiere al menos $0.50. Recarga en openrouter.ai/settings/credits.",
    "Crédit OpenRouter insuffisant. La reconnaissance vocale exige un solde d'au moins 0,50 $. Rechargez sur openrouter.ai/settings/credits.",
    "Nicht genug OpenRouter-Guthaben. Die Spracherkennung erfordert mindestens 0,50 $. Lade unter openrouter.ai/settings/credits auf.",
]);

pub const HTTP_403: Msg = Msg([
    "请求被拒绝（可能触发了内容审核或 Key 权限限制）：{0}",
    "請求被拒絕（可能觸發了內容審核或 Key 權限限制）：{0}",
    "Request refused (content moderation or key permissions): {0}",
    "リクエストが拒否されました（コンテンツ審査またはキーの権限制限）：{0}",
    "요청이 거부되었습니다(콘텐츠 검토 또는 키 권한 제한): {0}",
    "Solicitud rechazada (moderación de contenido o permisos de la clave): {0}",
    "Requête refusée (modération du contenu ou droits de la clé) : {0}",
    "Anfrage abgelehnt (Inhaltsmoderation oder Schlüsselberechtigungen): {0}",
]);

pub const HTTP_404: Msg = Msg([
    "找不到模型或接口，请检查模型名称：{0}",
    "找不到模型或介面，請檢查模型名稱：{0}",
    "Model or endpoint not found. Check the model name: {0}",
    "モデルまたはエンドポイントが見つかりません。モデル名を確認してください：{0}",
    "모델 또는 엔드포인트를 찾을 수 없습니다. 모델 이름을 확인하세요: {0}",
    "No se encontró el modelo o el endpoint. Revisa el nombre del modelo: {0}",
    "Modèle ou point d'accès introuvable. Vérifiez le nom du modèle : {0}",
    "Modell oder Endpunkt nicht gefunden. Prüfe den Modellnamen: {0}",
]);

pub const HTTP_TIMEOUT: Msg = Msg([
    "上游模型响应超时，请稍后重试或换一个更快的模型。",
    "上游模型回應逾時，請稍後重試或換一個更快的模型。",
    "The model took too long to respond. Try again or pick a faster model.",
    "モデルの応答がタイムアウトしました。しばらくして再試行するか、より高速なモデルを選んでください。",
    "모델 응답 시간이 초과되었습니다. 잠시 후 다시 시도하거나 더 빠른 모델을 선택하세요.",
    "El modelo tardó demasiado en responder. Inténtalo de nuevo o elige un modelo más rápido.",
    "Le modèle a mis trop de temps à répondre. Réessayez ou choisissez un modèle plus rapide.",
    "Das Modell hat zu lange gebraucht. Versuche es erneut oder wähle ein schnelleres Modell.",
]);

pub const HTTP_429: Msg = Msg([
    "请求过于频繁或免费额度已用完，请稍后重试。",
    "請求過於頻繁或免費額度已用完，請稍後重試。",
    "Too many requests, or the free quota is used up. Try again shortly.",
    "リクエストが多すぎるか、無料枠を使い切りました。しばらくして再試行してください。",
    "요청이 너무 많거나 무료 한도를 모두 사용했습니다. 잠시 후 다시 시도하세요.",
    "Demasiadas solicitudes o se agotó la cuota gratuita. Inténtalo de nuevo en breve.",
    "Trop de requêtes, ou quota gratuit épuisé. Réessayez dans un instant.",
    "Zu viele Anfragen oder das Gratiskontingent ist aufgebraucht. Versuche es gleich noch einmal.",
]);

pub const STAGE_FAILED: Msg = Msg([
    "{0}失败：{1}",
    "{0}失敗：{1}",
    "{0} failed: {1}",
    "{0}に失敗しました：{1}",
    "{0} 실패: {1}",
    "{0} falló: {1}",
    "{0} : échec. {1}",
    "{0} fehlgeschlagen: {1}",
]);

pub const STAGE_TEST: Msg = Msg([
    "连接测试",
    "連線測試",
    "Connection test",
    "接続テスト",
    "연결 테스트",
    "Prueba de conexión",
    "Test de connexion",
    "Verbindungstest",
]);

pub const STAGE_STT: Msg = Msg([
    "语音识别",
    "語音辨識",
    "Speech recognition",
    "音声認識",
    "음성 인식",
    "Reconocimiento de voz",
    "Reconnaissance vocale",
    "Spracherkennung",
]);

pub const STAGE_POLISH: Msg = Msg([
    "文本润色",
    "文字潤飾",
    "Polishing",
    "テキスト整形",
    "텍스트 다듬기",
    "Pulido del texto",
    "Mise en forme du texte",
    "Textglättung",
]);

pub const CUSTOM_ENDPOINT: Msg = Msg([
    "自定义接口",
    "自訂介面",
    "Custom endpoint",
    "カスタムエンドポイント",
    "사용자 지정 엔드포인트",
    "Endpoint personalizado",
    "Point d'accès personnalisé",
    "Eigener Endpunkt",
]);

pub const NEED_KEY: Msg = Msg([
    "请先填写 OpenRouter API Key。",
    "請先填寫 OpenRouter API Key。",
    "Enter your OpenRouter API key first.",
    "先に OpenRouter API キーを入力してください。",
    "먼저 OpenRouter API 키를 입력하세요.",
    "Primero introduce tu clave de API de OpenRouter.",
    "Saisissez d'abord votre clé API OpenRouter.",
    "Gib zuerst deinen OpenRouter-API-Schlüssel ein.",
]);

pub const NEED_KEY_IN_SETTINGS: Msg = Msg([
    "请先在设置中填写 OpenRouter API Key。",
    "請先在設定中填寫 OpenRouter API Key。",
    "Add your OpenRouter API key in Settings first.",
    "先に設定で OpenRouter API キーを入力してください。",
    "먼저 설정에서 OpenRouter API 키를 입력하세요.",
    "Primero añade tu clave de API de OpenRouter en Ajustes.",
    "Ajoutez d'abord votre clé API OpenRouter dans les Réglages.",
    "Hinterlege zuerst deinen OpenRouter-API-Schlüssel in den Einstellungen.",
]);

// ---------------------------------------------------------------------------
// Audio
// ---------------------------------------------------------------------------

pub const AUDIO_THREAD_EXITED: Msg = Msg([
    "录音线程异常退出。",
    "錄音執行緒異常結束。",
    "The recording thread stopped unexpectedly.",
    "録音スレッドが異常終了しました。",
    "녹음 스레드가 비정상적으로 종료되었습니다.",
    "El hilo de grabación se detuvo inesperadamente.",
    "Le thread d'enregistrement s'est arrêté de façon inattendue.",
    "Der Aufnahme-Thread wurde unerwartet beendet.",
]);

pub const AUDIO_THREAD_SPAWN: Msg = Msg([
    "无法启动录音线程：{0}",
    "無法啟動錄音執行緒：{0}",
    "Could not start the recording thread: {0}",
    "録音スレッドを開始できません：{0}",
    "녹음 스레드를 시작할 수 없습니다: {0}",
    "No se pudo iniciar el hilo de grabación: {0}",
    "Impossible de démarrer le thread d'enregistrement : {0}",
    "Aufnahme-Thread konnte nicht gestartet werden: {0}",
]);

pub const AUDIO_START_FAILED: Msg = Msg([
    "录音线程启动失败。",
    "錄音執行緒啟動失敗。",
    "Recording failed to start.",
    "録音を開始できませんでした。",
    "녹음을 시작하지 못했습니다.",
    "No se pudo iniciar la grabación.",
    "Impossible de démarrer l'enregistrement.",
    "Aufnahme konnte nicht gestartet werden.",
]);

pub const NO_MIC: Msg = Msg([
    "没有找到可用的麦克风，请检查设备连接和系统麦克风权限。",
    "找不到可用的麥克風，請檢查裝置連線和系統麥克風權限。",
    "No microphone found. Check that one is connected and that Windows allows microphone access.",
    "使用できるマイクが見つかりません。接続とシステムのマイク許可を確認してください。",
    "사용 가능한 마이크가 없습니다. 장치 연결과 시스템 마이크 권한을 확인하세요.",
    "No se encontró ningún micrófono. Comprueba la conexión y los permisos de micrófono del sistema.",
    "Aucun micro trouvé. Vérifiez le branchement et l'autorisation d'accès au micro dans Windows.",
    "Kein Mikrofon gefunden. Prüfe den Anschluss und die Mikrofonberechtigung in Windows.",
]);

pub const UNSUPPORTED_FORMAT: Msg = Msg([
    "暂不支持当前麦克风采样格式：{0}",
    "暫不支援目前麥克風的取樣格式：{0}",
    "This microphone's sample format is not supported yet: {0}",
    "このマイクのサンプル形式には未対応です：{0}",
    "이 마이크의 샘플 형식은 아직 지원되지 않습니다: {0}",
    "El formato de muestreo de este micrófono aún no es compatible: {0}",
    "Le format d'échantillonnage de ce micro n'est pas encore pris en charge : {0}",
    "Das Sample-Format dieses Mikrofons wird noch nicht unterstützt: {0}",
]);

pub const MIC_UNAVAILABLE: Msg = Msg([
    "麦克风不可用：{0}",
    "麥克風無法使用：{0}",
    "Microphone unavailable: {0}",
    "マイクを使用できません：{0}",
    "마이크를 사용할 수 없습니다: {0}",
    "Micrófono no disponible: {0}",
    "Micro indisponible : {0}",
    "Mikrofon nicht verfügbar: {0}",
]);

// ---------------------------------------------------------------------------
// App, shortcuts and recording flow
// ---------------------------------------------------------------------------

pub const INVALID_SHORTCUT: Msg = Msg([
    "无效的快捷键“{0}”：{1}",
    "無效的快速鍵「{0}」：{1}",
    "Invalid shortcut “{0}”: {1}",
    "無効なショートカット「{0}」：{1}",
    "잘못된 단축키 “{0}”: {1}",
    "Atajo no válido «{0}»: {1}",
    "Raccourci invalide « {0} » : {1}",
    "Ungültiges Tastenkürzel „{0}“: {1}",
]);

pub const SHORTCUT_TAKEN: Msg = Msg([
    "快捷键 {0} 注册失败，可能已被其他程序占用：{1}",
    "快速鍵 {0} 註冊失敗，可能已被其他程式佔用：{1}",
    "Could not register the shortcut {0}. Another app may be using it: {1}",
    "ショートカット {0} を登録できません。他のアプリが使用している可能性があります：{1}",
    "단축키 {0}을(를) 등록하지 못했습니다. 다른 앱이 사용 중일 수 있습니다: {1}",
    "No se pudo registrar el atajo {0}. Puede que otra app lo esté usando: {1}",
    "Impossible d'enregistrer le raccourci {0}. Une autre app l'utilise peut-être : {1}",
    "Tastenkürzel {0} konnte nicht registriert werden. Möglicherweise nutzt es eine andere App: {1}",
]);

pub const HTTPS_ONLY: Msg = Msg([
    "只允许打开 https 链接。",
    "只允許開啟 https 連結。",
    "Only https links can be opened.",
    "https のリンクのみ開けます。",
    "https 링크만 열 수 있습니다.",
    "Solo se pueden abrir enlaces https.",
    "Seuls les liens https peuvent être ouverts.",
    "Es können nur https-Links geöffnet werden.",
]);

pub const LOCK_RECORDING: Msg = Msg([
    "录音状态锁定失败。",
    "錄音狀態鎖定失敗。",
    "Could not access the recording state.",
    "録音状態にアクセスできません。",
    "녹음 상태에 접근할 수 없습니다.",
    "No se pudo acceder al estado de grabación.",
    "Impossible d'accéder à l'état de l'enregistrement.",
    "Auf den Aufnahmestatus konnte nicht zugegriffen werden.",
]);

pub const LOCK_SHORTCUT: Msg = Msg([
    "快捷键状态锁定失败。",
    "快速鍵狀態鎖定失敗。",
    "Could not access the shortcut state.",
    "ショートカットの状態にアクセスできません。",
    "단축키 상태에 접근할 수 없습니다.",
    "No se pudo acceder al estado del atajo.",
    "Impossible d'accéder à l'état du raccourci.",
    "Auf den Tastenkürzel-Status konnte nicht zugegriffen werden.",
]);

pub const LISTENING_TOGGLE: Msg = Msg([
    "正在聆听，再按一次快捷键结束，Esc 取消。",
    "正在聆聽，再按一次快速鍵結束，Esc 取消。",
    "Listening. Press the shortcut again to finish, Esc to cancel.",
    "聞き取り中。もう一度ショートカットで終了、Esc でキャンセル。",
    "듣는 중입니다. 단축키를 다시 누르면 끝나고 Esc로 취소합니다.",
    "Escuchando. Pulsa el atajo otra vez para terminar o Esc para cancelar.",
    "À l'écoute. Appuyez à nouveau sur le raccourci pour terminer, Échap pour annuler.",
    "Hört zu. Kürzel erneut drücken zum Beenden, Esc zum Abbrechen.",
]);

pub const LISTENING_HOLD: Msg = Msg([
    "正在聆听，松开快捷键结束，Esc 取消。",
    "正在聆聽，放開快速鍵結束，Esc 取消。",
    "Listening. Release the shortcut to finish, Esc to cancel.",
    "聞き取り中。ショートカットを離すと終了、Esc でキャンセル。",
    "듣는 중입니다. 단축키를 놓으면 끝나고 Esc로 취소합니다.",
    "Escuchando. Suelta el atajo para terminar o Esc para cancelar.",
    "À l'écoute. Relâchez le raccourci pour terminer, Échap pour annuler.",
    "Hört zu. Kürzel loslassen zum Beenden, Esc zum Abbrechen.",
]);

pub const RECOGNIZING: Msg = Msg([
    "正在识别…",
    "正在辨識…",
    "Transcribing…",
    "認識中…",
    "인식 중…",
    "Transcribiendo…",
    "Transcription…",
    "Wird erkannt…",
]);

pub const PHASE_PREPARE: Msg = Msg([
    "准备音频",
    "準備音訊",
    "Preparing audio",
    "音声を準備中",
    "오디오 준비 중",
    "Preparando audio",
    "Préparation de l'audio",
    "Audio wird vorbereitet",
]);

pub const PHASE_STT: Msg = Msg([
    "语音识别",
    "語音辨識",
    "Transcribing",
    "音声認識中",
    "음성 인식 중",
    "Transcribiendo",
    "Transcription",
    "Transkription",
]);

pub const PHASE_POLISH: Msg = Msg([
    "智能润色",
    "智慧潤飾",
    "Polishing",
    "整形中",
    "다듬는 중",
    "Puliendo",
    "Mise en forme",
    "Wird geglättet",
]);

pub const PHASE_OUTPUT: Msg = Msg([
    "输出文本",
    "輸出文字",
    "Typing",
    "入力中",
    "입력 중",
    "Escribiendo",
    "Saisie",
    "Wird eingefügt",
]);

pub const PHASE_DONE: Msg = Msg([
    "完成", "完成", "Done", "完了", "완료", "Listo", "Terminé", "Fertig",
]);

pub const TOO_SHORT: Msg = Msg([
    "录音太短，已忽略。",
    "錄音太短，已忽略。",
    "Recording too short, skipped.",
    "録音が短すぎるため無視しました。",
    "녹음이 너무 짧아 무시했습니다.",
    "Grabación demasiado corta, se omitió.",
    "Enregistrement trop court, ignoré.",
    "Aufnahme zu kurz, übersprungen.",
]);

pub const NO_SOUND: Msg = Msg([
    "没有检测到声音，请检查麦克风是否静音或选错设备。",
    "沒有偵測到聲音，請檢查麥克風是否靜音或選錯裝置。",
    "No sound detected. Check that the mic isn't muted and the right device is selected.",
    "音声が検出されませんでした。マイクのミュートや選択デバイスを確認してください。",
    "소리가 감지되지 않았습니다. 마이크 음소거 여부와 선택한 장치를 확인하세요.",
    "No se detectó sonido. Comprueba que el micrófono no esté silenciado y que el dispositivo sea el correcto.",
    "Aucun son détecté. Vérifiez que le micro n'est pas coupé et que le bon appareil est sélectionné.",
    "Kein Ton erkannt. Prüfe, ob das Mikrofon stummgeschaltet oder das falsche Gerät gewählt ist.",
]);

pub const POLISH_FAILED: Msg = Msg([
    "润色失败，已输出原始识别：{0}",
    "潤飾失敗，已輸出原始辨識結果：{0}",
    "Polishing failed, so the raw transcript was used: {0}",
    "整形に失敗したため、認識結果をそのまま出力しました：{0}",
    "다듬기에 실패해 원본 인식 결과를 출력했습니다: {0}",
    "Falló el pulido; se usó la transcripción original: {0}",
    "La mise en forme a échoué, la transcription brute a été utilisée : {0}",
    "Glättung fehlgeschlagen, das Rohtranskript wurde verwendet: {0}",
]);

pub const INSERTED: Msg = Msg([
    "已输入。",
    "已輸入。",
    "Typed.",
    "入力しました。",
    "입력했습니다.",
    "Escrito.",
    "Saisi.",
    "Eingefügt.",
]);

pub const NO_TEXT_FIELD: Msg = Msg([
    "没有找到输入框，点击复制。",
    "找不到輸入框，點擊複製。",
    "No text field found. Click to copy.",
    "入力欄が見つかりません。クリックでコピー。",
    "입력란을 찾지 못했습니다. 클릭하여 복사하세요.",
    "No se encontró un campo de texto. Haz clic para copiar.",
    "Aucun champ de texte trouvé. Cliquez pour copier.",
    "Kein Textfeld gefunden. Zum Kopieren klicken.",
]);

pub const CANCELLED: Msg = Msg([
    "录音已取消。",
    "錄音已取消。",
    "Recording cancelled.",
    "録音をキャンセルしました。",
    "녹음을 취소했습니다.",
    "Grabación cancelada.",
    "Enregistrement annulé.",
    "Aufnahme abgebrochen.",
]);

// ---------------------------------------------------------------------------
// Tray
// ---------------------------------------------------------------------------

pub const TRAY_SHOW: Msg = Msg([
    "打开主界面",
    "開啟主視窗",
    "Open Sayso",
    "メイン画面を開く",
    "메인 창 열기",
    "Abrir Sayso",
    "Ouvrir Sayso",
    "Sayso öffnen",
]);

pub const TRAY_TOGGLE: Msg = Msg([
    "开始 / 结束录音",
    "開始 / 結束錄音",
    "Start / stop dictation",
    "録音の開始 / 終了",
    "녹음 시작 / 종료",
    "Iniciar / detener dictado",
    "Démarrer / arrêter la dictée",
    "Diktat starten / beenden",
]);

pub const TRAY_QUIT: Msg = Msg([
    "退出", "結束", "Quit", "終了", "종료", "Salir", "Quitter", "Beenden",
]);

pub const TRAY_TOOLTIP: Msg = Msg([
    "Sayso 语音输入",
    "Sayso 語音輸入",
    "Sayso voice typing",
    "Sayso 音声入力",
    "Sayso 음성 입력",
    "Sayso: dictado por voz",
    "Sayso – dictée vocale",
    "Sayso Spracheingabe",
]);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tags() {
        assert_eq!(parse("zh-CN"), Some(Locale::ZhCn));
        assert_eq!(parse("zh-Hans"), Some(Locale::ZhCn));
        assert_eq!(parse("zh-TW"), Some(Locale::ZhTw));
        assert_eq!(parse("zh_HK"), Some(Locale::ZhTw));
        assert_eq!(parse("en-US"), Some(Locale::En));
        assert_eq!(parse("de"), Some(Locale::De));
        assert_eq!(parse("auto"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn fills_placeholders() {
        set(Locale::En);
        assert_eq!(trf(&STAGE_FAILED, &[&"Polishing", &"boom"]), "Polishing failed: boom");
    }
}
