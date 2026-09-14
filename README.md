# Download with yt-dlp — Windows Share target

A native Windows Share target and companion Rust CLI that download shared web links with [yt-dlp](https://github.com/yt-dlp/yt-dlp). The Share target opens a visible PowerShell window and saves media in the Windows Downloads known folder.

Media and subtitles are downloaded in separate passes. Missing or broken subtitle tracks therefore cannot turn a successful media download into a failure.

## How activation works

Windows does not pass a shared link as an ordinary command-line argument. The packaged GUI executable reads `AppInstance::GetActivatedEventArgs`, casts a `ShareTarget` activation to `ShareTargetActivatedEventArgs`, and retrieves the `DataPackageView` as `WebLink`, legacy `Uri`, or text.

The project deliberately builds two executables from one Rust library:

- `share-to-ytdlp-share-target.exe` is a windowless GUI-subsystem executable registered in `AppxManifest.xml`. `build.ps1` copies it into the loose package as `ShareToYtDlp.exe`.
- `share-to-ytdlp.exe` is a normal console CLI with `--help`, `--version`, and `download <URL>`.

Both paths launch the same `package/download.ps1` implementation. The Appx manifest controls Share registration and its displayed PNG assets; `resources/app.rc` embeds the conventional Win32 manifest and version metadata into the Rust executables.

## Requirements

- Windows 10 version 2004 or newer, or Windows 11
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) available on `PATH`
- PowerShell 7 (`pwsh`) available on `PATH`; Windows PowerShell is used as a fallback
- FFmpeg available to yt-dlp for merging and metadata embedding
- A current Rust MSVC toolchain
- Visual Studio Build Tools and a Windows SDK for the MSVC linker and resource compiler
- Developer Mode enabled in Windows settings

## Install

Clone the repository, then run from PowerShell:

```powershell
.\install.ps1
```

The installer builds the Rust executables and registers the package manifest locally. The Share target is a development-mode loose package, so keep the cloned directory in place after installation.

Close and reopen an existing Share panel after installation. **Download with yt-dlp** may initially appear at the bottom of the **Share using** list.

## Usage

From an app or website that uses the native Windows Share dialog:

1. Share a link.
2. Choose **Download with yt-dlp**.
3. Follow progress in the PowerShell window.

The console interface exposes the same downloader:

```powershell
cargo run -- --help
cargo run -- --version
cargo run -- download 'https://x.com/example/status/123'
```

Downloads go to the Windows Downloads known folder by default.

## Configuration

To override the destination, put an absolute folder path in:

```text
%LOCALAPPDATA%\ShareToYtDlp\output-directory.txt
```

To use browser cookies, put one of `brave`, `chrome`, `edge`, or `firefox` in:

```text
%LOCALAPPDATA%\ShareToYtDlp\cookies-browser.txt
```

## Optional Explorer context-menu command

`youtubedownloadhere-current-user.reg` adds **Download clipboard here** to a folder background menu for the current user without requiring administrator access.

`youtubedownloadhere.reg` installs the equivalent machine-wide `HKEY_CLASSES_ROOT` entry and normally requires administrator access.

Both variants use the same two-pass media/subtitle behavior.

## Build and test

Build the release package layout:

```powershell
.\build.ps1
```

Run the complete template-derived quality gate:

```powershell
.\check-all.ps1
```

The downloader tests use a fake yt-dlp executable and make no network requests.

## Uninstall

```powershell
.\uninstall.ps1
```

## Previous C++ implementation

The final C++ implementation is preserved by the `cpp-v1.0.1` Git tag.

## License

This project is licensed under the [Mozilla Public License 2.0](LICENSE).
