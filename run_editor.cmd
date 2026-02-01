@echo off
cd /d D:\workspace\bevy_ai_editor
title Bevy AI Editor
echo ==========================================
echo Starting Bevy AI Editor...
echo ==========================================
cargo run -p bevy_ai_editor
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Error occurred!
    pause
)
