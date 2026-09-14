@echo off
echo %*>>"%FAKE_YTDLP_LOG%"
echo %* | %SystemRoot%\System32\findstr.exe /C:"--skip-download" >nul
if %errorlevel%==0 exit /b %FAKE_YTDLP_SUBTITLE_EXIT%
exit /b %FAKE_YTDLP_MEDIA_EXIT%
