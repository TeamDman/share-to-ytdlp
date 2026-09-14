use crate::downloader::launch_downloader;
use crate::url::extract_url;
use eyre::Context;
use eyre::Result;
use eyre::bail;
use windows::ApplicationModel::Activation::ActivationKind;
use windows::ApplicationModel::Activation::ShareTargetActivatedEventArgs;
use windows::ApplicationModel::AppInstance;
use windows::ApplicationModel::DataTransfer::DataPackageView;
use windows::ApplicationModel::DataTransfer::StandardDataFormats;
use windows::Foundation::Uri;
use windows::Win32::System::WinRT::RO_INIT_MULTITHREADED;
use windows::Win32::System::WinRT::RoInitialize;
use windows::Win32::System::WinRT::RoUninitialize;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR;
use windows::Win32::UI::WindowsAndMessaging::MB_ICONINFORMATION;
use windows::Win32::UI::WindowsAndMessaging::MB_OK;
use windows::Win32::UI::WindowsAndMessaging::MessageBoxW;
use windows::core::HSTRING;
use windows::core::Interface;
use windows::core::PCWSTR;

#[derive(Debug)]
struct WinRtApartment;

impl WinRtApartment {
    fn initialize() -> Result<Self> {
        // SAFETY: This is the first WinRT call on the dedicated GUI entry thread.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
            .wrap_err("failed to initialize the Windows Runtime")?;
        Ok(Self)
    }
}

impl Drop for WinRtApartment {
    fn drop(&mut self) {
        // SAFETY: Every successful RoInitialize call must be balanced on the same thread.
        unsafe { RoUninitialize() };
    }
}

/// Runs the windowless executable registered by the package as a Windows Share target.
///
/// # Errors
///
/// Returns an error if activation metadata cannot be read or the Share operation cannot be reported.
pub fn run() -> Result<()> {
    let _apartment = WinRtApartment::initialize()?;
    let activated = AppInstance::GetActivatedEventArgs()
        .wrap_err("failed to read the packaged app activation arguments")?;

    if activated.Kind()? != ActivationKind::ShareTarget {
        show_instructions();
        return Ok(());
    }

    let arguments: ShareTargetActivatedEventArgs = activated
        .cast()
        .wrap_err("Share activation arguments had the wrong WinRT type")?;
    handle_share(&arguments)
}

fn handle_share(arguments: &ShareTargetActivatedEventArgs) -> Result<()> {
    let operation = arguments.ShareOperation()?;
    operation.ReportStarted()?;

    let result = (|| -> Result<()> {
        let data = operation.Data()?;
        let shared_url = get_shared_url(&data)?;
        let url = validate_web_address(&shared_url)?;
        operation.ReportDataRetrieved()?;
        let _process_id = launch_downloader(&url)?;
        operation.ReportCompleted()?;
        Ok(())
    })();

    if let Err(error) = result {
        let message = HSTRING::from(format!("{error:#}"));
        operation.ReportError(&message)?;
    }

    Ok(())
}

fn get_shared_url(data: &DataPackageView) -> Result<String> {
    let web_link = StandardDataFormats::WebLink()?;
    if data.Contains(&web_link)? {
        return Ok(data.GetWebLinkAsync()?.join()?.AbsoluteUri()?.to_string());
    }

    // Retained for older Share sources; WebLink is preferred by newer apps.
    let uri = StandardDataFormats::Uri()?;
    if data.Contains(&uri)? {
        return Ok(data.GetUriAsync()?.join()?.AbsoluteUri()?.to_string());
    }

    let text = StandardDataFormats::Text()?;
    if data.Contains(&text)? {
        let shared_text = data.GetTextAsync()?.join()?.to_string();
        return extract_url(&shared_text)
            .ok_or_else(|| eyre::eyre!("the shared text did not contain an HTTP or HTTPS URL"));
    }

    bail!("the shared item did not contain a web link or text")
}

fn validate_web_address(candidate: &str) -> Result<String> {
    let uri = Uri::CreateUri(&HSTRING::from(candidate)).wrap_err("the shared link was invalid")?;
    let scheme = uri.SchemeName()?.to_string();
    if !matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https") {
        bail!("the shared link did not use HTTP or HTTPS");
    }
    if uri.Host()?.is_empty() {
        bail!("the shared link did not contain a host name");
    }
    Ok(uri.AbsoluteUri()?.to_string())
}

fn show_instructions() {
    show_message(
        "Share a web link to 'Download with yt-dlp' from the Windows Share dialog.\n\nDownloads are saved to your Windows Downloads folder. You can override it with:\n%LOCALAPPDATA%\\ShareToYtDlp\\output-directory.txt",
        MB_ICONINFORMATION,
    );
}

/// Shows a fatal startup error from the windowless Share-target executable.
pub fn show_fatal_error(error: &str) {
    show_message(error, MB_ICONERROR);
}

fn show_message(message: &str, icon: windows::Win32::UI::WindowsAndMessaging::MESSAGEBOX_STYLE) {
    let message = HSTRING::from(message);
    let title = HSTRING::from("Download with yt-dlp");
    // SAFETY: Both HSTRING buffers remain alive for the synchronous MessageBoxW call.
    let _ = unsafe {
        MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            PCWSTR(title.as_ptr()),
            MB_OK | icon,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_an_x_link_with_winrt() {
        let _apartment = WinRtApartment::initialize().expect("WinRT should initialize");
        let url = validate_web_address("https://x.com/example/status/123")
            .expect("X URL should be valid");
        assert_eq!(url, "https://x.com/example/status/123");
    }

    #[test]
    fn rejects_non_web_schemes() {
        let _apartment = WinRtApartment::initialize().expect("WinRT should initialize");
        assert!(validate_web_address("file:///C:/video.mp4").is_err());
    }
}
