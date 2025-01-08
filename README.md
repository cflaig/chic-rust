# Chic

A simple Interface for Chess written in Rust using [Slint](https://slint.rs/) for the user interface.

## About

This project is designed to visualize a chessboard. It is built as a beginner-level Rust project to explore GUI development concepts in **Rust** and practice coding with the Slint framework.
## Usage

1. Install Rust by following its [getting-started guide](https://www.rust-lang.org/learn/get-started).
   Once this is done, you should have the `rustc` compiler and the `cargo` build system installed in your `PATH`. Ensure that you have Rust installed on your system. You can verify the installation by running `cargo --version` and `rustc --version` in your terminal.
2. Clone the [repository](https://github.com/cflaig/chess-ui) or download and extract the [ZIP archive of this repository](https://github.com/cflaig/chess-ui/archive/refs/heads/main.zip).
3. Change into the Project:
    ```bash
    cd chess-rust  
    ```
4. Build with `cargo`:
    ```bash
    cargo build
    ```
5. Run the application binary:
    ```bash
    cargo run
    ```

## Running in a Browser

1. Install `wasm-pack` with Cargo:
   ```bash
   cargo install wasm-pack
   ```
2. Build the `chic-rust` project for the `wasm32` target:
   ```bash
   wasm-pack build --release --target web
   ```
3. Host the project using a web server:
   Browsers don't allow loading JavaScript modules from the `file://` protocol. You can use Python to serve the `index.html` file:
   ```bash
   python3 -m http.server
   ```
   Once started, the project will be accessible in your browser at [http://localhost:8000](http://localhost:8000).


## Features
- 🏁 Display a chessboard based on a FEN string.
- ✨ Uses Scalable Vector Graphics (SVG) for piece images for a sharp and clean interface.
- 🎨 Dynamic and responsive UI layout using Slint for smooth experience.

Future plans include adding move generation, game state validation, and a simple engine.
## Next Steps

Here are some planned features for future iterations:
1. **Move Generation**: Add logic for generating legal moves based on the current board state.
2. **Basic Engine Support**: Integrate a simple chess engine for automated move suggestions.
3. **Database Features**:
   - Save/load games in a database.
   - Replay stored games directly via the graphical interface.

4. **User Interaction**: Allow users to make moves via the GUI to play against the engine or explore scenarios.

## Credits

Image resource: https://github.com/jontejj/chess-svg