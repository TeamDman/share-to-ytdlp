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
use windows::Win32::Foundation::APPMODEL_ERROR_NO_PACKAGE;
use windows::Win32::Foundation::ERROR_INSUFFICIENT_BUFFER;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
use windows::Win32::System::Console::FreeConsole;
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

/// Handles a packaged activation when the zero-argument CLI was launched by Windows.
///
/// # Errors
///
/// Returns an error if package identity or activation metadata cannot be read, or the Share operation cannot be reported.
pub fn try_handle_activation() -> Result<bool> {
    if !has_package_identity()? {
        return Ok(false);
    }

    // The one executable is a console binary so CLI help/version work normally.
    // Packaged zero-argument launches are UI activations, so detach the otherwise
    // unused console before WinRT opens the actual downloader window.
    // SAFETY: Detaching the current process from its console takes no pointers.
    let _ = unsafe { FreeConsole() };

    let _apartment = WinRtApartment::initialize()?;
    let activated = AppInstance::GetActivatedEventArgs()
        .wrap_err("failed to read the packaged app activation arguments")?;

    if activated.Kind()? != ActivationKind::ShareTarget {
        show_instructions();
        return Ok(true);
    }

    let arguments: ShareTargetActivatedEventArgs = activated
        .cast()
        .wrap_err("Share activation arguments had the wrong WinRT type")?;
    handle_share(&arguments)?;
    Ok(true)
}

fn has_package_identity() -> Result<bool> {
    let mut length = 0_u32;
    // SAFETY: The documented sizing call accepts a null output buffer and writes
    // only the required character count to `length`.
    let status = unsafe { GetCurrentPackageFullName(&raw mut length, None) };
    match status {
        APPMODEL_ERROR_NO_PACKAGE => Ok(false),
        ERROR_INSUFFICIENT_BUFFER | ERROR_SUCCESS => Ok(true),
        other => bail!(
            "GetCurrentPackageFullName failed with Win32 error {}",
            other.0
        ),
    }
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

/// Shows a fatal packaged-activation error.
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

    #[test]
    fn test_process_has_no_package_identity() {
        assert!(!has_package_identity().expect("package identity check should succeed"));
    }
}
