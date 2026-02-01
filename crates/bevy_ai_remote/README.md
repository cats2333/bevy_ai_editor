# Bevy AI Remote

This is the companion plugin for **Axiom**, the AI-Powered Bevy Editor.
It enables your Bevy game to communicate with the Axiom editor via the Bevy Remote Protocol (BRP), allowing for:

- Real-time entity spawning and manipulation.
- Hot asset uploading (GLB models, textures) from Editor to Game.
- Scene clearing and management.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
bevy_ai_remote = "0.1.0" # Check crates.io for latest version
```

## Usage

Simply add the plugin to your App:

```rust
use bevy::prelude::*;
use bevy_ai_remote::BevyAiRemotePlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the AI Remote Plugin
        .add_plugins(BevyAiRemotePlugin)
        .run();
}
```

This will open an HTTP server on `127.0.0.1:15721` (default BRP port) that the Axiom Editor connects to.

## Features

- **Asset Uploading**: Automatically handles Base64 encoded assets sent from Axiom and saves them to `assets/_remote_cache/`.
- **Smart Loading**: Automatically loads GLB files as Scenes.
- **Cleanup**: Provides tools to clear the scene (filtering for generated assets).

For the full editor experience, visit the [Axiom Repository](https://github.com/cats2333/bevy_ai_editor).
