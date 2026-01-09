# Gemini Project: sona

## Project Overview

This project, named "sona," is a Rust application built using the Bevy game engine. Based on the dependencies listed in `Cargo.toml` (`bevy`, `midly`, `oxisynth`, `serde`, `toml`), it appears to be a music or audio-related application, possibly a synthesizer or a simple digital audio workstation (DAW). The presence of `keybindings.toml` suggests an interactive application with user controls for navigation and playback. The current implementation in `src/main.rs` is a minimal Bevy application that displays a single red square, serving as a basic scaffold.

## Building and Running

As a standard Rust project, the following `cargo` commands can be used:

*   **Build the project:**
    ```sh
    cargo build
    ```

*   **Run the project:**
    ```sh
    cargo run
    ```

*   **Run tests:**
    ```sh
    cargo test
    ```

## Development Conventions

The code in `src/main.rs` follows standard Rust and Bevy conventions. The project uses a `keybindings.toml` file for user-configurable keybindings, which are loaded and parsed by the application. Any new features should aim to integrate with the existing Bevy ECS (Entity Component System) architecture.
