@echo off
cd /d D:\workspace\bevy_ai_editor
echo ==========================================
echo Launching Axiom Editor and Simple Game...
echo ==========================================

:: Start Game first to let it bind the port
start "Simple Game" run_game.cmd

:: Wait a bit for game to initialize
timeout /t 3 /nobreak >nul

:: Start Editor
start "Bevy AI Editor" run_editor.cmd

echo Done. You can close this window.
timeout /t 2 /nobreak >nul
