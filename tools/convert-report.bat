@echo off
rem ============================================================
rem  2-Pyramid drag-and-drop conversion + report  (ASCII only!)
rem
rem  This launcher is intentionally ASCII-only: cmd.exe parses
rem  .bat files using the OEM codepage, so any UTF-8 text here
rem  would be mis-decoded and executed as garbage commands.
rem  All user-facing text lives in convert-report.ps1 instead.
rem
rem  Usage: drag a resource pack (.zip / .mcpack) or a folder
rem         onto this file. Optional switches are forwarded:
rem           -Target 1.21.4 | -Yes | -NoOpen
rem ============================================================

setlocal
chcp 65001 >nul 2>nul
set "PS="
where pwsh >nul 2>nul && set "PS=pwsh"
if not defined PS (
    where powershell >nul 2>nul && set "PS=powershell"
)
if not defined PS (
    echo [ERROR] PowerShell not found. Please install PowerShell 7 or use Windows PowerShell.
    pause
    exit /b 3
)

"%PS%" -NoProfile -ExecutionPolicy Bypass -File "%~dp0convert-report.ps1" %*
set "RC=%ERRORLEVEL%"

rem No pause here: the .ps1 pauses itself only when a real console is
rem attached, so redirected/non-interactive runs never hang.
exit /b %RC%
