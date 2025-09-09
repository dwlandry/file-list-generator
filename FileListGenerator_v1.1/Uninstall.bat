@echo off
color 0E
cls
title File List Generator - Uninstall

echo.
echo  ==============================================================
echo    REMOVING FILE LIST GENERATOR
echo    From Right-Click Menu
echo  ==============================================================
echo.
echo  This will remove "Generate File List" from your
echo  right-click menu.
echo.
echo  The program files will NOT be deleted.
echo.
echo  Press any key to uninstall...
pause >nul

echo.
echo  Removing from right-click menu...
echo.

REM Remove from folder context menu
reg delete "HKEY_CURRENT_USER\Software\Classes\Directory\shell\FileListGenerator" /f >nul 2>&1

REM Remove from folder background context menu
reg delete "HKEY_CURRENT_USER\Software\Classes\Directory\Background\shell\FileListGenerator" /f >nul 2>&1

REM Remove from drive context menu
reg delete "HKEY_CURRENT_USER\Software\Classes\Drive\shell\FileListGenerator" /f >nul 2>&1

color 0A
echo.
echo  ==============================================================
echo    UNINSTALL COMPLETE
echo  ==============================================================
echo.
echo  The right-click menu entries have been removed.
echo.
echo  To reinstall: Run Install.bat
echo.
pause
exit /b 0