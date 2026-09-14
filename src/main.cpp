#include <windows.h>

#include <cwctype>
#include <filesystem>
#include <string>
#include <vector>

#include <winrt/base.h>
#include <winrt/Windows.ApplicationModel.h>
#include <winrt/Windows.ApplicationModel.Activation.h>
#include <winrt/Windows.ApplicationModel.DataTransfer.h>
#include <winrt/Windows.ApplicationModel.DataTransfer.ShareTarget.h>
#include <winrt/Windows.Foundation.h>

namespace fs = std::filesystem;
namespace activation = winrt::Windows::ApplicationModel::Activation;
namespace data_transfer = winrt::Windows::ApplicationModel::DataTransfer;
namespace share_target = winrt::Windows::ApplicationModel::DataTransfer::ShareTarget;

namespace
{
    std::wstring trim(std::wstring value)
    {
        const auto first = value.find_first_not_of(L" \t\r\n");
        if (first == std::wstring::npos)
        {
            return {};
        }

        const auto last = value.find_last_not_of(L" \t\r\n");
        return value.substr(first, last - first + 1);
    }

    bool starts_with_http(const std::wstring& value)
    {
        return value.rfind(L"https://", 0) == 0 || value.rfind(L"http://", 0) == 0;
    }

    std::wstring extract_url(std::wstring value)
    {
        value = trim(std::move(value));
        if (starts_with_http(value) && value.find_first_of(L" \t\r\n") == std::wstring::npos)
        {
            return value;
        }

        auto position = value.find(L"https://");
        if (position == std::wstring::npos)
        {
            position = value.find(L"http://");
        }
        if (position == std::wstring::npos)
        {
            return {};
        }

        auto end = value.find_first_of(L" \t\r\n\"'<>﻿", position);
        auto candidate = value.substr(position, end == std::wstring::npos ? end : end - position);
        while (!candidate.empty() && std::wstring_view(L".,;!)]}").find(candidate.back()) != std::wstring_view::npos)
        {
            candidate.pop_back();
        }
        return candidate;
    }

    std::wstring get_shared_url(const data_transfer::DataPackageView& data)
    {
        if (data.Contains(data_transfer::StandardDataFormats::WebLink()))
        {
            if (auto uri = data.GetWebLinkAsync().get())
            {
                return uri.AbsoluteUri().c_str();
            }
        }

        // Uri is retained for older share sources; WebLink is preferred by newer apps.
        if (data.Contains(data_transfer::StandardDataFormats::Uri()))
        {
            if (auto uri = data.GetUriAsync().get())
            {
                return uri.AbsoluteUri().c_str();
            }
        }

        if (data.Contains(data_transfer::StandardDataFormats::Text()))
        {
            return extract_url(data.GetTextAsync().get().c_str());
        }

        return {};
    }

    fs::path module_directory()
    {
        std::vector<wchar_t> buffer(32768);
        const DWORD length = GetModuleFileNameW(nullptr, buffer.data(), static_cast<DWORD>(buffer.size()));
        if (length == 0 || length >= buffer.size())
        {
            winrt::throw_last_error();
        }
        return fs::path(std::wstring(buffer.data(), length)).parent_path();
    }

    // Apply the quoting rules used by CommandLineToArgvW. CreateProcess receives
    // this command line directly, so URL characters such as &, ;, and $ are data,
    // not shell syntax.
    std::wstring quote_argument(std::wstring_view argument)
    {
        std::wstring result(1, L'"');
        size_t backslashes = 0;
        for (const wchar_t character : argument)
        {
            if (character == L'\\')
            {
                ++backslashes;
            }
            else if (character == L'"')
            {
                result.append(backslashes * 2 + 1, L'\\');
                result.push_back(L'"');
                backslashes = 0;
            }
            else
            {
                result.append(backslashes, L'\\');
                backslashes = 0;
                result.push_back(character);
            }
        }
        result.append(backslashes * 2, L'\\');
        result.push_back(L'"');
        return result;
    }

    std::wstring find_powershell()
    {
        std::vector<wchar_t> buffer(32768);
        const DWORD length = SearchPathW(nullptr, L"pwsh.exe", nullptr, static_cast<DWORD>(buffer.size()), buffer.data(), nullptr);
        if (length > 0 && length < buffer.size())
        {
            return std::wstring(buffer.data(), length);
        }

        wchar_t windows_directory[MAX_PATH]{};
        const UINT windows_length = GetWindowsDirectoryW(windows_directory, MAX_PATH);
        if (windows_length == 0 || windows_length >= MAX_PATH)
        {
            winrt::throw_last_error();
        }
        return (fs::path(windows_directory) / L"System32" / L"WindowsPowerShell" / L"v1.0" / L"powershell.exe").wstring();
    }

    void launch_downloader(const std::wstring& url)
    {
        const auto script_path = module_directory() / L"download.ps1";
        if (!fs::exists(script_path))
        {
            throw std::runtime_error("download.ps1 is missing from the app package");
        }

        const auto powershell = find_powershell();
        std::wstring command_line = quote_argument(powershell) +
            L" -NoLogo -NoProfile -ExecutionPolicy Bypass -File " + quote_argument(script_path.wstring()) +
            L" -Url " + quote_argument(url);
        std::vector<wchar_t> mutable_command(command_line.begin(), command_line.end());
        mutable_command.push_back(L'\0');

        STARTUPINFOW startup_info{};
        startup_info.cb = sizeof(startup_info);
        PROCESS_INFORMATION process_info{};
        if (!CreateProcessW(
                powershell.c_str(),
                mutable_command.data(),
                nullptr,
                nullptr,
                FALSE,
                CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT,
                nullptr,
                nullptr,
                &startup_info,
                &process_info))
        {
            winrt::throw_last_error();
        }

        CloseHandle(process_info.hThread);
        CloseHandle(process_info.hProcess);
    }

    void handle_share(const activation::ShareTargetActivatedEventArgs& arguments)
    {
        const share_target::ShareOperation operation = arguments.ShareOperation();
        operation.ReportStarted();

        try
        {
            const auto url = get_shared_url(operation.Data());
            if (url.empty() || !starts_with_http(url))
            {
                operation.ReportError(L"The shared item did not contain an HTTP or HTTPS link.");
                return;
            }

            // Confirm that Windows can parse the candidate before handing it to yt-dlp.
            const winrt::Windows::Foundation::Uri parsed(url);
            if (parsed.Host().empty())
            {
                operation.ReportError(L"The shared link was not a valid web address.");
                return;
            }

            operation.ReportDataRetrieved();
            launch_downloader(url);
            operation.ReportCompleted();
        }
        catch (const winrt::hresult_error& error)
        {
            operation.ReportError(error.message());
        }
        catch (const std::exception& error)
        {
            operation.ReportError(winrt::to_hstring(error.what()));
        }
    }
}

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR, int)
{
    try
    {
        winrt::init_apartment(winrt::apartment_type::multi_threaded);
        const auto arguments = winrt::Windows::ApplicationModel::AppInstance::GetActivatedEventArgs();
        if (arguments && arguments.Kind() == activation::ActivationKind::ShareTarget)
        {
            handle_share(arguments.as<activation::ShareTargetActivatedEventArgs>());
            return 0;
        }

        MessageBoxW(
            nullptr,
            L"Share a web link to 'Download with yt-dlp' from the Windows Share dialog.\n\n"
            L"Downloads are saved to your Windows Downloads folder. You can override it by putting a path in:\n"
            L"%LOCALAPPDATA%\\ShareToYtDlp\\output-directory.txt",
            L"Download with yt-dlp",
            MB_OK | MB_ICONINFORMATION);
        return 0;
    }
    catch (const winrt::hresult_error& error)
    {
        MessageBoxW(nullptr, error.message().c_str(), L"Download with yt-dlp", MB_OK | MB_ICONERROR);
        return 1;
    }
    catch (const std::exception& error)
    {
        MessageBoxA(nullptr, error.what(), "Download with yt-dlp", MB_OK | MB_ICONERROR);
        return 1;
    }
}
