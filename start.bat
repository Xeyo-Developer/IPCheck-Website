@echo off
REM Check for cargo existance
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo ERROR: Cargo not found! Install rust first!!!
    pause
    exit /b 1
)

REM Try to kill process on port 3000 (safer approach)
for /f "tokens=5" %%a in ('netstat -ano ^| findstr /R /C:":3000 *LISTENING"') do (
    echo Found process on port 3000, attempting to terminate...
    taskkill /F /PID %%a >nul 2>nul
    if %errorlevel% equ 0 (
        echo Successfully terminated process on port 3000
    ) else (
        echo No process was using port 3000
    )
)

REM Run project
echo Starting server...
cargo run
if %errorlevel% neq 0 (
    echo FATAL ERROR: Server failed to start!!!
    pause
    exit /b 1
)

REM This will never execute because cargo run blocks
echo Done!!!
pause