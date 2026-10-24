@echo off

if exist "%~dp0version.dll" (
    start "" rundll32.exe "%~dp0version.dll",EntryPoint
) else (
    start "" rundll32.exe "%~dp0target\release\version.dll",EntryPoint
)
exit
