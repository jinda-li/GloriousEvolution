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
    if !settings.auto_paste || text.is_empty() {
        return Ok(TextDelivery::NeedsCopy);
    }

    #[cfg(target_os = "windows")]
    {
        return windows_input::type_text(text);
    }

    #[cfg(not(target_os = "windows"))]
    Ok(TextDelivery::NeedsCopy)
}

#[cfg(target_os = "windows")]
mod windows_input {
    use std::time::{Duration, Instant};

    use windows::Win32::{
        Foundation::HWND,
        System::Threading::GetCurrentProcessId,
        UI::{
            Input::KeyboardAndMouse::{
                GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
                KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VIRTUAL_KEY, VK_CONTROL,
                VK_LWIN, VK_MENU, VK_RETURN, VK_RWIN, VK_SHIFT,
            },
            WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId, IsWindowVisible},
        },
    };

    use super::TextDelivery;
    use crate::AppResult;

    /// SendInput batches beyond this size are occasionally truncated by busy targets.
    const CHUNK_UNITS: usize = 200;

    pub fn type_text(text: &str) -> AppResult<TextDelivery> {
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

        // A still-held Alt/Ctrl from the hotkey would turn typed characters into shortcuts.
        wait_for_modifiers_released(Duration::from_millis(1500));

        let normalized = text.replace("\r\n", "\n");
        let mut inputs = Vec::with_capacity(normalized.len() * 2);
        for ch in normalized.chars() {
            if ch == '\n' {
                // Shift+Enter inserts a line break in chat apps instead of sending.
                inputs.push(vk_input(VK_SHIFT, false));
                inputs.push(vk_input(VK_RETURN, false));
                inputs.push(vk_input(VK_RETURN, true));
                inputs.push(vk_input(VK_SHIFT, true));
                continue;
            }
            let mut buf = [0u16; 2];
            for unit in ch.encode_utf16(&mut buf) {
                inputs.push(unicode_input(*unit, false));
                inputs.push(unicode_input(*unit, true));
            }
        }

        for chunk in inputs.chunks(CHUNK_UNITS) {
            let sent = unsafe { SendInput(chunk, std::mem::size_of::<INPUT>() as i32) };
            if sent != chunk.len() as u32 {
                return Ok(TextDelivery::NeedsCopy);
            }
            if inputs.len() > CHUNK_UNITS {
                std::thread::sleep(Duration::from_millis(8));
            }
        }
        Ok(TextDelivery::Inserted)
    }

    fn wait_for_modifiers_released(timeout: Duration) {
        let started = Instant::now();
        let modifiers = [VK_MENU, VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN];
        while started.elapsed() < timeout {
            let held = modifiers
                .iter()
                .any(|key| unsafe { GetAsyncKeyState(key.0 as i32) } as u16 & 0x8000 != 0);
            if !held {
                return;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
    }

    fn keyboard(vk: VIRTUAL_KEY, scan: u16, flags: KEYBD_EVENT_FLAGS) -> INPUT {
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: scan,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    fn unicode_input(code_unit: u16, key_up: bool) -> INPUT {
        let mut flags = KEYEVENTF_UNICODE;
        if key_up {
            flags |= KEYEVENTF_KEYUP;
        }
        keyboard(VIRTUAL_KEY(0), code_unit, flags)
    }

    fn vk_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
        let flags = if key_up {
            KEYEVENTF_KEYUP
        } else {
            KEYBD_EVENT_FLAGS(0)
        };
        keyboard(vk, 0, flags)
    }
}
