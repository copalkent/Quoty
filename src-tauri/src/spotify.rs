//! Spotify detection and media controls on Windows.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpotifyStatus {
    pub is_running: bool,
    pub is_playing: bool,
    pub artist: String,
    pub title: String,
}

#[cfg(windows)]
extern "system" {
    fn keybd_event(b_vk: u8, b_scan: u8, dw_flags: u32, dw_extra_info: usize);
}

const VK_MEDIA_NEXT_TRACK: u8 = 0xB0;
const VK_MEDIA_PREV_TRACK: u8 = 0xB1;
const VK_MEDIA_PLAY_PAUSE: u8 = 0xB3;
const KEYEVENTF_KEYUP: u32 = 0x0002;

#[cfg(windows)]
fn send_media_key(key: u8) {
    unsafe {
        keybd_event(key, 0, 0, 0);
        keybd_event(key, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub fn play_pause() {
    #[cfg(windows)]
    send_media_key(VK_MEDIA_PLAY_PAUSE);
}

pub fn next_track() {
    #[cfg(windows)]
    send_media_key(VK_MEDIA_NEXT_TRACK);
}

pub fn prev_track() {
    #[cfg(windows)]
    send_media_key(VK_MEDIA_PREV_TRACK);
}

pub fn launch_spotify() {
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("cmd")
            .args(["/c", "start", "spotify:"])
            .spawn();
    }
}

#[cfg(windows)]
pub fn current_status() -> SpotifyStatus {
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_FORMAT,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        IsWindowVisible,
    };

    struct Finder {
        status: Option<SpotifyStatus>,
    }

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let finder = &mut *(lparam.0 as *mut Finder);

        if !IsWindowVisible(hwnd).as_bool() {
            return BOOL(1);
        }

        let length = GetWindowTextLengthW(hwnd);
        if length <= 0 {
            return BOOL(1);
        }

        let mut buffer = vec![0u16; (length + 1) as usize];
        let copied = GetWindowTextW(hwnd, &mut buffer);
        if copied <= 0 {
            return BOOL(1);
        }

        let title = String::from_utf16_lossy(&buffer[..copied as usize]);

        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));

        let is_spotify = if let Ok(proc_handle) =
            OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
        {
            let mut img_buf = [0u16; 1024];
            let mut img_size = 1024u32;
            let ok = QueryFullProcessImageNameW(
                proc_handle,
                PROCESS_NAME_FORMAT(0),
                windows::core::PWSTR(img_buf.as_mut_ptr()),
                &mut img_size,
            );
            if ok.is_ok() && img_size > 0 {
                let path = String::from_utf16_lossy(&img_buf[..img_size as usize]).to_lowercase();
                path.ends_with("spotify.exe")
            } else {
                false
            }
        } else {
            false
        };

        if is_spotify {
            if title == "Spotify"
                || title == "Spotify Free"
                || title == "Spotify Premium"
                || title == "Default IME"
                || title.is_empty()
            {
                if finder.status.is_none() {
                    finder.status = Some(SpotifyStatus {
                        is_running: true,
                        is_playing: false,
                        artist: String::new(),
                        title: String::new(),
                    });
                }
            } else if let Some((artist, track)) = title.split_once(" - ") {
                finder.status = Some(SpotifyStatus {
                    is_running: true,
                    is_playing: true,
                    artist: artist.trim().to_string(),
                    title: track.trim().to_string(),
                });
                return BOOL(0); // Stop enum once active song is found
            }
        }

        BOOL(1)
    }

    let mut finder = Finder { status: None };
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut finder as *mut _ as isize));
    }

    finder.status.unwrap_or_default()
}

#[cfg(not(windows))]
pub fn current_status() -> SpotifyStatus {
    SpotifyStatus::default()
}
