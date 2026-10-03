@echo off
chcp 65001 >nul
setlocal enabledelayedexpansion
title 2-Pyramid 转换 + 报告

rem ============================================================
rem  2-Pyramid 拖放转换脚本
rem
rem  用法：把一个资源包（.zip / .mcpack）或含资源包的文件夹
rem        直接拖到本脚本上；也可以双击运行后手动输入路径。
rem
rem  行为：跑与 GUI 完全相同的转换管线 → 输出结构化 JSON 报告
rem        （结构分析 + 纯转换/总时间 + 逐任务画像 + 体积变化）
rem        并自动打开报告所在文件夹。
rem ============================================================

set "SCRIPT_DIR=%~dp0"
set "REPO_DIR=%SCRIPT_DIR%.."
set "TARGET_PATH="

rem ── 1) 收集拖入的路径（支持多个） ─────────────────────────────
if not "%~1"=="" (
    set "TARGET_PATH=%~1"
) else (
    echo.
    echo 请把资源包（.zip / .mcpack）或文件夹拖到本脚本上，
    echo 或在此粘贴路径后回车：
    echo.
    set /p "TARGET_PATH=路径: "
)

if "%TARGET_PATH%"=="" (
    echo [错误] 没有提供路径。
    pause
    exit /b 2
)

rem 去掉可能的引号
set "TARGET_PATH=%TARGET_PATH:"=%"

if not exist "%TARGET_PATH%" (
    echo [错误] 路径不存在：%TARGET_PATH%
    pause
    exit /b 2
)

rem ── 2) 找到 2-pyramid.exe ─────────────────────────────────────
set "EXE="
for %%P in (
    "%LOCALAPPDATA%\2-Pyramid\2-pyramid.exe"
    "%LOCALAPPDATA%\2-Pyramid-Beta\2-pyramid.exe"
    "%REPO_DIR%\src-tauri\target\release\2-pyramid.exe"
    "%REPO_DIR%\src-tauri\target\debug\2-pyramid.exe"
    "%REPO_DIR%\release\staging\2-pyramid.exe"
) do (
    if not defined EXE if exist "%%~P" set "EXE=%%~P"
)

if not defined EXE (
    echo [错误] 找不到 2-pyramid.exe。
    echo         请先安装 2-Pyramid，或在仓库里先跑一次构建。
    pause
    exit /b 3
)
echo 使用程序：%EXE%

rem ── 3) 选择目标版本 ───────────────────────────────────────────
echo.
echo 选择目标版本（输入序号，或直接输入版本号 / pack_format）：
echo.
echo   1) 1.20-1.20.1      15) 1.21.5
echo   2) 1.20.2           16) 1.21.6
echo   3) 1.20.3-1.20.4    17) 1.21.7-1.21.8
echo   4) 1.20.5-1.20.6    18) 1.21.9-1.21.10
echo   5) 1.21-1.21.1      19) 1.21.11
echo   6) 1.21.2-1.21.3    20) 26.1-26.1.2
echo   7) 1.21.4           21) 26.2
echo   8) 26.3（最新，默认）
echo   9) bedrock（实验性，仅测试）
echo.
set "CHOICE="
set /p "CHOICE=选择 [8]: "
if "!CHOICE!"=="" set "CHOICE=8"

set "TARGET="
if "!CHOICE!"=="1" set "TARGET=1.20-1.20.1"
if "!CHOICE!"=="2" set "TARGET=1.20.2"
if "!CHOICE!"=="3" set "TARGET=1.20.3-1.20.4"
if "!CHOICE!"=="4" set "TARGET=1.20.5-1.20.6"
if "!CHOICE!"=="5" set "TARGET=1.21-1.21.1"
if "!CHOICE!"=="6" set "TARGET=1.21.2-1.21.3"
if "!CHOICE!"=="7" set "TARGET=1.21.4"
if "!CHOICE!"=="8" set "TARGET=26.3"
if "!CHOICE!"=="9" set "TARGET=bedrock"
if not defined TARGET set "TARGET=!CHOICE!"

if /i "!TARGET!"=="bedrock" (
    echo.
    echo [警告] Bedrock 转换尚未完成、存在严重问题，仅用于测试。
    set /p "OK=仍要继续？(y/N): "
    if /i not "!OK!"=="y" (
        echo 已取消。
        pause
        exit /b 0
    )
)

rem ── 4) 准备报告目录 ───────────────────────────────────────────
rem 报告放在被处理对象旁边：文件 → 同目录；文件夹 → 文件夹内
set "REPORT_DIR="
if exist "%TARGET_PATH%\" (
    set "REPORT_DIR=%TARGET_PATH%"
) else (
    for %%F in ("%TARGET_PATH%") do set "REPORT_DIR=%%~dpF"
)
set "STAMP=%DATE:~0,4%%DATE:~5,2%%DATE:~8,2%-%TIME:~0,2%%TIME:~3,2%%TIME:~6,2%"
set "STAMP=%STAMP: =0%"
set "REPORT=%REPORT_DIR%2pyr-report-%STAMP%.json"

echo.
echo ============================================================
echo  开始转换
echo ============================================================
echo.

rem ── 5) 跑转换 ─────────────────────────────────────────────────
"%EXE%" --convert "%TARGET_PATH%" --to "!TARGET!" --report "%REPORT%"
set "RC=%ERRORLEVEL%"

echo.
if "%RC%"=="0" (
    echo [完成] 转换成功。
) else (
    echo [注意] 有资源包转换失败（退出码 %RC%），详见上面的输出与报告。
)

if exist "%REPORT%" (
    echo 报告：%REPORT%
    echo 正在打开报告所在文件夹…
    start "" "%REPORT_DIR%"
) else (
    echo [提示] 未生成报告文件。
)

echo.
pause
endlocal
