@echo off
cd /d "%~dp0"
call build.bat
if errorlevel 1 (
    echo.
    echo Сборка не удалась.
    pause
    exit /b 1
)
echo.
pause
