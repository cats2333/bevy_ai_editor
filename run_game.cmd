@echo off
cd /d D:\workspace\bevy_ai_editor
title Simple Game (Bevy 0.18)
echo ==========================================
echo Starting Simple Game Host...
echo ==========================================
cargo run -p simple_game
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Error occurred!
    pause
)
