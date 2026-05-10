use crate::{settings::AppSettings, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDelivery {
    Inserted,
    NeedsCopy,
}

pub fn write_clipboard(text: &str) -> AppResult<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text.to_string())?;
    Ok(())
}

pub fn deliver_text(settings: &AppSettings, text: &str) -> AppResult<TextDelivery> {
    if !settings.auto_paste {
        return Ok(TextDelivery::NeedsCopy);
    }

    #[cfg(target_os = "windows")]
    {
        return direct_insert_windows(text);
    }

    #[cfg(not(target_os = "windows"))]
    Ok(TextDelivery::NeedsCopy)
}

#[cfg(target_os = "windows")]
fn direct_insert_windows(text: &str) -> AppResult<TextDelivery> {
    use windows::Win32::{
        Foundation::HWND,
        System::Threading::GetCurrentProcessId,
        UI::{
            Input::KeyboardAndMouse::{
                SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
                KEYEVENTF_UNICODE, VIRTUAL_KEY,
            },
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId, IsWindowVisible},
        },
    };

    let foreground = unsafe { GetForegroundWindow() };
    if foreground == HWND(0) || !unsafe { IsWindowVisible(foreground).as_bool() } {
        return Ok(TextDelivery::NeedsCopy);
    }

    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(foreground, Some(&mut process_id));
    }
    if process_id == unsafe { GetCurrentProcessId() } {
        return Ok(TextDelivery::NeedsCopy);
    }

    let mut inputs = Vec::with_capacity(text.encode_utf16().count() * 2);
    for code_unit in text.encode_utf16() {
        inputs.push(unicode_input(code_unit, false));
        inputs.push(unicode_input(code_unit, true));
    }

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    return if sent == inputs.len() as u32 {
        Ok(TextDelivery::Inserted)
    } else {
        Ok(TextDelivery::NeedsCopy)
    };

    fn unicode_input(code_unit: u16, key_up: bool) -> INPUT {
        let mut flags = KEYEVENTF_UNICODE;
        if key_up {
            flags |= KEYEVENTF_KEYUP;
        }

        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: VIRTUAL_KEY(0),
                    wScan: code_unit,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }
}
