#[cfg(windows)]
use crate::url::require_http_url;
#[cfg(windows)]
use eyre::Context;
use eyre::Result;
#[cfg(windows)]
use eyre::bail;
#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::PathBuf;

/// Starts the packaged PowerShell downloader in a new visible console window.
///
/// # Errors
///
/// Returns an error if the URL is invalid, the script or PowerShell cannot be found, or the child process cannot be started.
#[cfg(windows)]
pub fn launch_downloader(url: &str) -> Result<u32> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::CREATE_NEW_CONSOLE;
    use windows::Win32::System::Threading::CREATE_UNICODE_ENVIRONMENT;
    use windows::Win32::System::Threading::CreateProcessW;
    use windows::Win32::System::Threading::PROCESS_INFORMATION;
    use windows::core::PCWSTR;
    use windows::core::PWSTR;

    let url = require_http_url(url)?;
    let script = find_download_script()?;
    let powershell = find_powershell()?;
    let arguments = [
        powershell.as_os_str(),
        OsStr::new("-NoLogo"),
        OsStr::new("-NoProfile"),
        OsStr::new("-ExecutionPolicy"),
        OsStr::new("Bypass"),
        OsStr::new("-File"),
        script.as_os_str(),
        OsStr::new("-Url"),
        OsStr::new(&url),
    ];
    let application_name = wide_null(powershell.as_os_str());
    let mut command_line = build_command_line(&arguments);

    // Do not set STARTF_USESTDHANDLES. The packaged parent has detached from its
    // activation console, so forwarding its standard handles would create a
    // visible but blank PowerShell window. With zero startup flags and a new
    // console, Windows assigns PowerShell fresh console input/output handles.
    let startup_info = startup_info_for_new_console();
    let mut process_info = PROCESS_INFORMATION::default();
    let creation_flags = CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT;

    // SAFETY: The application and mutable command-line buffers are terminated
    // and remain alive for the call. Security/environment/current-directory
    // pointers are null, handles are not inherited, and both output structures
    // are valid for writes.
    unsafe {
        CreateProcessW(
            PCWSTR(application_name.as_ptr()),
            Some(PWSTR(command_line.as_mut_ptr())),
            None,
            None,
            false,
            creation_flags,
            None,
            PCWSTR::null(),
            &raw const startup_info,
            &raw mut process_info,
        )
    }
    .wrap_err_with(|| format!("failed to start {}", powershell.display()))?;

    let process_id = process_info.dwProcessId;
    // SAFETY: CreateProcessW returned both owned handles successfully. Closing
    // them does not terminate the independently running PowerShell process.
    let close_thread = unsafe { CloseHandle(process_info.hThread) };
    // SAFETY: This is the second distinct owned handle returned by CreateProcessW.
    let close_process = unsafe { CloseHandle(process_info.hProcess) };
    close_thread.wrap_err("failed to close the downloader thread handle")?;
    close_process.wrap_err("failed to close the downloader process handle")?;

    Ok(process_id)
}

/// Reports that this Windows integration is unavailable on other platforms.
///
/// # Errors
///
/// Always returns an error outside Windows.
#[cfg(not(windows))]
pub fn launch_downloader(_url: &str) -> Result<u32> {
    eyre::bail!("share-to-ytdlp is supported only on Windows")
}

#[cfg(windows)]
fn wide_null(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn startup_info_for_new_console() -> windows::Win32::System::Threading::STARTUPINFOW {
    use windows::Win32::System::Threading::STARTUPINFOW;

    STARTUPINFOW {
        cb: u32::try_from(std::mem::size_of::<STARTUPINFOW>())
            .expect("STARTUPINFOW size should fit in u32"),
        ..STARTUPINFOW::default()
    }
}

#[cfg(windows)]
fn build_command_line(arguments: &[&OsStr]) -> Vec<u16> {
    let mut result = Vec::new();
    for (index, argument) in arguments.iter().enumerate() {
        if index > 0 {
            result.push(u16::from(b' '));
        }
        result.extend(quote_windows_argument(argument));
    }
    result.push(0);
    result
}

/// Quotes one argument according to the rules consumed by `CommandLineToArgvW`.
#[cfg(windows)]
fn quote_windows_argument(argument: &OsStr) -> Vec<u16> {
    let mut result = vec![u16::from(b'"')];
    let mut backslashes = 0_usize;

    for character in argument.encode_wide() {
        if character == u16::from(b'\\') {
            backslashes += 1;
        } else if character == u16::from(b'"') {
            result.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes * 2 + 1));
            result.push(u16::from(b'"'));
            backslashes = 0;
        } else {
            result.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
            backslashes = 0;
            result.push(character);
        }
    }

    result.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes * 2));
    result.push(u16::from(b'"'));
    result
}

#[cfg(windows)]
fn find_download_script() -> Result<PathBuf> {
    let executable = std::env::current_exe().wrap_err("failed to locate the running executable")?;
    if let Some(sibling) = executable
        .parent()
        .map(|parent| parent.join("download.ps1"))
        && sibling.is_file()
    {
        return Ok(sibling);
    }

    let development_copy = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("package")
        .join("download.ps1");
    if development_copy.is_file() {
        return Ok(development_copy);
    }

    bail!("download.ps1 is missing beside the executable and from the package directory")
}

#[cfg(windows)]
fn find_powershell() -> Result<PathBuf> {
    if let Some(path) = find_on_path("pwsh.exe") {
        return Ok(path);
    }

    if let Some(program_files) = std::env::var_os("ProgramFiles") {
        let path = PathBuf::from(program_files)
            .join("PowerShell")
            .join("7")
            .join("pwsh.exe");
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Some(windows_directory) = std::env::var_os("SystemRoot") {
        let path = PathBuf::from(windows_directory)
            .join("System32")
            .join("WindowsPowerShell")
            .join("v1.0")
            .join("powershell.exe");
        if path.is_file() {
            return Ok(path);
        }
    }

    bail!("neither PowerShell 7 nor Windows PowerShell could be found")
}

#[cfg(windows)]
fn find_on_path(filename: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(filename))
        .find(|candidate| candidate.is_file())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    fn quoted(argument: &str) -> String {
        String::from_utf16(&quote_windows_argument(OsStr::new(argument)))
            .expect("quoted argument should remain valid UTF-16")
    }

    #[test]
    fn quotes_spaces_quotes_and_trailing_backslashes() {
        assert_eq!(quoted("plain"), "\"plain\"");
        assert_eq!(quoted("hello world"), "\"hello world\"");
        assert_eq!(quoted("a\"b"), "\"a\\\"b\"");
        assert_eq!(quoted("C:\\path\\"), "\"C:\\path\\\\\"");
    }

    #[test]
    fn new_console_does_not_forward_detached_standard_handles() {
        use windows::Win32::System::Threading::STARTF_USESTDHANDLES;

        let startup_info = startup_info_for_new_console();
        assert!(!startup_info.dwFlags.contains(STARTF_USESTDHANDLES));
    }
}
