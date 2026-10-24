@echo off

if exist "%~dp0SSSniffer.dll" (
    start "" rundll32.exe "%~dp0SSSniffer.dll",EntryPoint
) else (
    start "" rundll32.exe "%~dp0target\release\SSSniffer.dll",EntryPoint
)
exit
