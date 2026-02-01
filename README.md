# Axiom: AI-Powered Bevy Editor

**Axiom** is a next-generation, AI-native editor for the [Bevy Game Engine](https://bevyengine.org/). 
It allows you to build game levels, spawn assets, and manipulate the scene using natural language commands, powered by LLMs (Gemini).

## 🚀 Key Features

*   **AI Architect**: Instruct the AI to "Build a 5x5 road grid" or "Place a forest here", and watch it happen instantly.
*   **Real-time Bridge**: Connects to your running Bevy game via **BRP (Bevy Remote Protocol)** over HTTP.
*   **Hot Asset Upload**: Upload `.glb` models and textures from the editor to the game runtime on the fly. No restart needed.
*   **Intelligent Tooling**: 
    *   **Road Engineer**: Specialized logic for procedural road generation (handling orientation, T-junctions, and bends automatically).
*   **Modern UI**: Built with `egui`, featuring a file tree, chat interface, and minimal toolbars.

## 🛠️ Architecture

*   **apps/axiom**: The Editor application (Rust + Egui + LLM Client).
*   **crates/bevy_ai_remote**: A Bevy Plugin that you add to your game to enable Axiom control.
*   **examples/simple_game**: A reference Bevy game project configured to work with Axiom.

## 🏁 Getting Started

### 1. Prerequisites
*   Rust (latest stable)
*   A Gemini API Key (or compatible OpenAI proxy)

### 2. Configuration
Copy the example environment file:
```bash
cp .env.example .env
```
Edit `.env` and paste your API Key:
```ini
GEMINI_API_KEY=your_key_here
# GEMINI_BASE_URL=... (Optional if using proxy)
```

### 3. Run Everything
We provide a script to launch both the Editor and the Game:

**Windows**:
```cmd
run_all.cmd
```

This will:
1.  Start `simple_game` (Listens on port 15721).
2.  Start `Axiom` Editor.

### 4. How to Use
1.  **Camera**: In the game window, use **WASD** to move and **Q/E** to fly up/down.
2.  **Select Assets**: In Axiom, expand `resources/models`. Check `road-straight.glb` etc. and click **"🚀 Ingest Context"**.
3.  **Command**: Type a command like:
    > "Generate a 5x5 Tian grid road network centered at 2,2. Strictly follow the Road Engineer rules to build the skeleton."
4.  **Magic**: The AI will analyze your request and execute batch commands to build the scene.

### 5. Adding to your own Bevy game
To use Axiom with your own project:
1. Add `bevy_ai_remote` to your dependencies in `Cargo.toml`:
   ```toml
   [dependencies]
   bevy_ai_remote = { path = "path/to/crates/bevy_ai_remote" } # Or git url
   ```
2. Add the plugin in your `main.rs`:
   ```rust
   app.add_plugins(bevy_ai_remote::BevyAiRemotePlugin);
   ```
3. Run your game!

## 🛣️ Road Engineer Rules
Axiom has built-in knowledge for Kenny Assets roads:
- **Grid Size**: 1.0
- **Straight**: X-Axis aligned.
- **Bend**: Connects North (-Z) and West (-X) at default rotation.
- **T-Junction**: Stem points South (+Z) at default rotation.

## 📝 License
MIT / Apache 2.0
