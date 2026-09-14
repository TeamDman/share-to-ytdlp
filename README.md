# Download with yt-dlp — Windows Share target

A small native Windows app that adds **Download with yt-dlp** to the Windows Share dialog for shared web links and text. It opens a visible PowerShell window and saves media in the Windows Downloads known folder.

Media and subtitles are downloaded in separate passes. Missing or broken subtitle tracks therefore cannot turn a successful media download into a failure.

## Requirements

- Windows 10 version 2004 or newer, or Windows 11
- [yt-dlp](https://github.com/yt-dlp/yt-dlp) available on `PATH`
- PowerShell 7 (`pwsh`) available on `PATH`
- FFmpeg available to yt-dlp for merging and metadata embedding
- Visual Studio 2022 with **Desktop development with C++** and a Windows 10/11 SDK
- Developer Mode enabled in Windows settings

## Install

Clone the repository, then run from PowerShell:

```powershell
.\install.ps1
```

The installer builds the native executable and registers its package manifest locally. The Share target is a development-mode loose package, so keep the cloned directory in place after installation.

Close and reopen an existing Share panel after installation. **Download with yt-dlp** may initially appear at the bottom of the **Share using** list.

## Usage

From an app or website that uses the native Windows Share dialog:

1. Share a link.
2. Choose **Download with yt-dlp**.
3. Follow progress in the PowerShell window.

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

Build the package layout:

```powershell
.\build.ps1
```

Run isolated downloader tests. They use a fake yt-dlp executable and make no network requests:

```powershell
.\tests\download.tests.ps1
```

Test the PowerShell downloader directly:

```powershell
.\package\download.ps1 -Url 'https://example.com/video' -NoPause
```

## Uninstall

```powershell
.\uninstall.ps1
```

## License

This project is licensed under the [Mozilla Public License 2.0](LICENSE).
