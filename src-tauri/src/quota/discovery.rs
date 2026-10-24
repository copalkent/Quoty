//! Finds the Antigravity language server on this machine.
//!
//! Antigravity (including the Antigravity IDE build) runs a local
//! `language_server_windows_*` helper. The helper is started by the IDE with a
//! `--csrf_token` argument and listens on loopback ports. We only read the
//! process command line and the TCP listener table: nothing on disk is touched,
//! no credentials are read, and the token is kept in memory only.

use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
pub struct Connection {
    pub port: u16,
    pub csrf: String,
    pub product: String,
}

impl Connection {
    pub fn source_label(&self) -> String {
        format!("{} · local language server", self.product)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Candidate {
    pub csrf: String,
    pub ports: Vec<u16>,
    pub product: String,
    pub started_at: u64,
}

/// Splits a Windows command line into arguments, honouring double quotes.
fn split_command_line(line: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;

    for character in line.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        args.push(current);
    }
    args
}

fn uuid_like(value: &str) -> bool {
    value.len() == 36
        && value
            .chars()
            .enumerate()
            .all(|(i, c)| match i {
                8 | 13 | 18 | 23 => c == '-',
                _ => c.is_ascii_hexdigit(),
            })
}

fn product_name(app_data_dir: &str) -> String {
    match app_data_dir.trim().to_ascii_lowercase().as_str() {
        "antigravity-ide" => "Antigravity IDE".to_string(),
        "antigravity" => "Antigravity".to_string(),
        other => {
            // Title-case the raw identifier as a fallback.
            other
                .split(['-', '_'])
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(|value| value.as_str())
}

/// Enumerates every local Antigravity language server that exposes a loopback
/// port and a session token. Order: most recently started first.
pub fn candidates() -> Vec<Candidate> {
    let mut found: Vec<Candidate> = Vec::new();

    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for (pid, process) in system.processes() {
        let name = process.name().to_string_lossy().to_ascii_lowercase();
        if !name.starts_with("language_server_windows_") {
            continue;
        }

        let exe = process
            .exe()
            .map(|path| path.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !exe.contains("antigravity") {
            continue;
        }

        // sysinfo can return an empty argument list when command-line access
        // is restricted; read the process parameters directly in that case.
        let args: Vec<String> = {
            let from_sysinfo: Vec<String> = process
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            if from_sysinfo.is_empty() {
                process_command_line(pid.as_u32())
                    .map(|line| split_command_line(&line))
                    .unwrap_or_default()
            } else {
                from_sysinfo
            }
        };

        let Some(csrf) = arg_value(&args, "--csrf_token").filter(|value| uuid_like(value)) else {
            continue;
        };
        let Some(app_data_dir) = arg_value(&args, "--app_data_dir") else {
            continue;
        };
        // Only accept the two known Antigravity data directories; other
        // language servers on the machine are out of scope.
        if !app_data_dir.to_ascii_lowercase().starts_with("antigravity") {
            continue;
        }

        let ports = loopback_listeners(pid.as_u32());
        if ports.is_empty() {
            continue;
        }

        found.push(Candidate {
            csrf: csrf.to_string(),
            ports,
            product: product_name(app_data_dir),
            started_at: process.start_time(),
        });
    }

    found.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    found
}

#[cfg(all(windows, target_pointer_width = "64"))]
fn process_command_line(pid: u32) -> Option<String> {
    use std::mem::{size_of, MaybeUninit};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_BASIC_INFORMATION, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };
    use windows::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};

    // PEB and RTL_USER_PROCESS_PARAMETERS offsets for 64-bit Windows.
    const PEB_PROCESS_PARAMETERS: usize = 0x20;
    const PARAMETERS_COMMAND_LINE: usize = 0x70;

    struct Handle(HANDLE);
    impl Drop for Handle {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct UnicodeString {
        length: u16,
        maximum_length: u16,
        buffer: *const u16,
    }

    unsafe fn read_bytes(handle: HANDLE, address: *const core::ffi::c_void, size: usize) -> Option<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        let mut read = 0usize;
        ReadProcessMemory(
            handle,
            address,
            buffer.as_mut_ptr().cast(),
            size,
            Some(&mut read),
        )
        .ok()?;
        if read != size {
            return None;
        }
        Some(buffer)
    }

    unsafe fn read_value<T: Copy>(handle: HANDLE, address: usize) -> Option<T> {
        let mut value = MaybeUninit::<T>::uninit();
        let mut read = 0usize;
        ReadProcessMemory(
            handle,
            address as *const core::ffi::c_void,
            value.as_mut_ptr().cast(),
            size_of::<T>(),
            Some(&mut read),
        )
        .ok()?;
        if read != size_of::<T>() {
            return None;
        }
        Some(value.assume_init())
    }

    let handle = Handle(unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ, false, pid).ok()?
    });

    let mut info = PROCESS_BASIC_INFORMATION::default();
    let mut returned = 0u32;
    let status = unsafe {
        NtQueryInformationProcess(
            handle.0,
            ProcessBasicInformation,
            &mut info as *mut _ as *mut core::ffi::c_void,
            size_of::<PROCESS_BASIC_INFORMATION>() as u32,
            &mut returned,
        )
    };
    if status.0 < 0 {
        return None;
    }

    let peb = info.PebBaseAddress as usize;
    if peb == 0 {
        return None;
    }

    let parameters: usize = unsafe { read_value(handle.0, peb + PEB_PROCESS_PARAMETERS)? };
    if parameters == 0 {
        return None;
    }

    let command_line: UnicodeString =
        unsafe { read_value(handle.0, parameters + PARAMETERS_COMMAND_LINE)? };
    if command_line.length == 0 || command_line.buffer.is_null() {
        return None;
    }

    let byte_length = (command_line.length as usize).min(64 * 1024);
    let bytes = unsafe {
        read_bytes(
            handle.0,
            command_line.buffer as *const core::ffi::c_void,
            byte_length,
        )?
    };
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_ne_bytes([pair[0], pair[1]]))
        .collect();

    Some(String::from_utf16_lossy(&units))
}

#[cfg(not(all(windows, target_pointer_width = "64")))]
fn process_command_line(_pid: u32) -> Option<String> {
    None
}

#[cfg(windows)]
fn loopback_listeners(pid: u32) -> Vec<u16> {
    use std::mem::size_of;
    use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    };

    const AF_INET: u32 = 2;

    let mut size: u32 = 0;
    // SAFETY: a null table pointer with a valid size pointer is the documented
    // way to query the required buffer size.
    let status = unsafe {
        GetExtendedTcpTable(
            None,
            &mut size,
            false,
            AF_INET,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    };
    if status != ERROR_INSUFFICIENT_BUFFER.0 || size == 0 {
        return Vec::new();
    }

    let mut buffer = vec![0u8; size as usize];
    // SAFETY: the buffer is at least `size` bytes, as reported above.
    let status = unsafe {
        GetExtendedTcpTable(
            Some(buffer.as_mut_ptr().cast()),
            &mut size,
            false,
            AF_INET,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        )
    };
    if status != 0 || size < size_of::<u32>() as u32 {
        return Vec::new();
    }

    // SAFETY: the API wrote a u32 row count followed by that many rows.
    let count = unsafe { std::ptr::read_unaligned(buffer.as_ptr() as *const u32) } as usize;
    let row_size = size_of::<MIB_TCPROW_OWNER_PID>();
    if buffer.len() < size_of::<u32>() + count * row_size {
        return Vec::new();
    }

    let mut ports = Vec::new();
    for index in 0..count {
        // SAFETY: bounds checked above; read_unaligned avoids alignment UB.
        let row = unsafe {
            std::ptr::read_unaligned(
                buffer
                    .as_ptr()
                    .add(size_of::<u32>() + index * row_size)
                    as *const MIB_TCPROW_OWNER_PID,
            )
        };
        if row.dwOwningPid != pid {
            continue;
        }
        // dwLocalAddr is in network byte order; only loopback listeners are used.
        let octets = row.dwLocalAddr.to_le_bytes();
        if !Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]).is_loopback() {
            continue;
        }
        let port = u16::from_be((row.dwLocalPort & 0xFFFF) as u16);
        if port != 0 {
            ports.push(port);
        }
    }

    ports.sort_unstable();
    ports.dedup();
    ports
}

#[cfg(not(windows))]
fn loopback_listeners(_pid: u32) -> Vec<u16> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_tokens() {
        assert!(uuid_like("dbab89af-ffe6-4951-908a-9b1f3b1f56c1"));
        assert!(!uuid_like("not-a-token"));
        assert!(!uuid_like("dbab89af-ffe6-4951-908a-9b1f3b1f56c1\n"));
        assert!(!uuid_like(""));
    }

    #[test]
    fn splits_windows_command_lines() {
        let args = split_command_line(
            r#""C:\Program Files\Antigravity\ls.exe" --csrf_token abc --app_data_dir antigravity-ide"#,
        );
        assert_eq!(
            args,
            vec![
                r"C:\Program Files\Antigravity\ls.exe",
                "--csrf_token",
                "abc",
                "--app_data_dir",
                "antigravity-ide",
            ]
        );
    }

    #[test]
    fn reads_flag_values() {
        let args = vec![
            "exe".to_string(),
            "--csrf_token".to_string(),
            "abc".to_string(),
            "--app_data_dir".to_string(),
            "antigravity-ide".to_string(),
        ];
        assert_eq!(arg_value(&args, "--csrf_token"), Some("abc"));
        assert_eq!(arg_value(&args, "--app_data_dir"), Some("antigravity-ide"));
        assert_eq!(arg_value(&args, "--missing"), None);
    }

    #[test]
    fn names_products() {
        assert_eq!(product_name("antigravity-ide"), "Antigravity IDE");
        assert_eq!(product_name("antigravity"), "Antigravity");
        assert_eq!(product_name("some_app"), "Some App");
    }
}
