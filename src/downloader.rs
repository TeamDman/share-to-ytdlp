#[cfg(windows)]
use crate::url::require_http_url;
#[cfg(windows)]
use eyre::Context;
use eyre::Result;
#[cfg(windows)]
use eyre::bail;
#[cfg(windows)]
use std::path::PathBuf;
#[cfg(windows)]
use std::process::Command;

/// Starts the packaged PowerShell downloader in a new visible console window.
///
/// # Errors
///
/// Returns an error if the URL is invalid, the script or PowerShell cannot be found, or the child process cannot be started.
#[cfg(windows)]
pub fn launch_downloader(url: &str) -> Result<u32> {
    use std::os::windows::process::CommandExt;
    use windows::Win32::System::Threading::CREATE_NEW_CONSOLE;
    use windows::Win32::System::Threading::CREATE_UNICODE_ENVIRONMENT;

    let url = require_http_url(url)?;
    let script = find_download_script()?;
    let powershell = find_powershell()?;
    let creation_flags = (CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT).0;

    let child = Command::new(&powershell)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&script)
        .arg("-Url")
        .arg(&url)
        .creation_flags(creation_flags)
        .spawn()
        .wrap_err_with(|| format!("failed to start {}", powershell.display()))?;

    Ok(child.id())
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
