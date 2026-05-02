@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0"
if exist rtuinventory.exe (
    rtuinventory.exe
) else (
    echo Error: rtuinventory.exe not found in current directory
    exit /b 1
)