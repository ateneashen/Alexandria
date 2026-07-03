@echo off
setlocal

REM Alexandria launcher for Windows
REM Usage: run.bat [scan <path> | serve | info]

set "EXE_DIR=%~dp0"

if defined CARGO_TARGET_DIR (
    set "EXE=%CARGO_TARGET_DIR%\release\alexandria.exe"
) else (
    set "EXE=%EXE_DIR%target\release\alexandria.exe"
)

if not exist "%EXE%" (
    echo Ejecutable no encontrado: %EXE%
    echo Compila primero con: cargo build --release
    exit /b 1
)

"%EXE%" %*
