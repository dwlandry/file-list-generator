@echo off
color 0B
cls
title File List Generator - Setup

echo.
echo  ==============================================================
echo    FILE LIST GENERATOR - ONE-TIME SETUP
echo    By David Landry - Version 1.1
echo  ==============================================================
echo.
echo  This will add "Generate File List" to your right-click menu
echo  You only need to do this ONCE!
echo.
echo  After setup:
echo   - Right-click any folder
echo   - Select "Generate File List"
echo   - That's it! No need to open this folder again
echo.
echo  Press any key to install...
pause >nul

REM Get the full path to the exe
set EXE_PATH=%~dp0FileListGenerator.exe

REM Check if exe exists
if not exist "%EXE_PATH%" (
    color 0C
    echo.
    echo  ERROR: Cannot find FileListGenerator.exe
    echo  Please make sure this file is in the same folder.
    echo.
    pause
    exit /b 1
)

echo.
echo  Installing to right-click menu...
echo.

REM Add to folder context menu
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\shell\FileListGenerator" /ve /d "Generate File List" /f >nul 2>&1
if errorlevel 1 goto :error
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\shell\FileListGenerator" /v "Icon" /t REG_SZ /d "%EXE_PATH%,0" /f >nul 2>&1
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\shell\FileListGenerator\command" /ve /d "\"%EXE_PATH%\" \"%%V\"" /f >nul 2>&1

REM Add to folder background context menu
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\Background\shell\FileListGenerator" /ve /d "Generate File List" /f >nul 2>&1
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\Background\shell\FileListGenerator" /v "Icon" /t REG_SZ /d "%EXE_PATH%,0" /f >nul 2>&1
reg add "HKEY_CURRENT_USER\Software\Classes\Directory\Background\shell\FileListGenerator\command" /ve /d "\"%EXE_PATH%\" \"%%V\"" /f >nul 2>&1

REM Add to drive context menu
reg add "HKEY_CURRENT_USER\Software\Classes\Drive\shell\FileListGenerator" /ve /d "Generate File List" /f >nul 2>&1
reg add "HKEY_CURRENT_USER\Software\Classes\Drive\shell\FileListGenerator" /v "Icon" /t REG_SZ /d "%EXE_PATH%,0" /f >nul 2>&1
reg add "HKEY_CURRENT_USER\Software\Classes\Drive\shell\FileListGenerator\command" /ve /d "\"%EXE_PATH%\" \"%%V\"" /f >nul 2>&1

color 0A
echo.
echo  ==============================================================
echo    SUCCESS! Installation Complete
echo  ==============================================================
echo.
echo  You can now close this window.
echo.
echo  HOW TO USE:
echo   1. Right-click any folder
echo   2. Select "Generate File List"
echo.
echo  To uninstall: Run Uninstall.bat
echo.
pause
exit /b 0

:error
color 0C
echo.
echo  ==============================================================
echo    ERROR: Installation Failed
echo  ==============================================================
echo.
echo  Please try:
echo   1. Run as Administrator (right-click, Run as administrator)
echo   2. Make sure Windows Defender isn't blocking the file
echo   3. Contact David Landry for support
echo.
pause
exit /b 1