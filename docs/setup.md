# Setup & Installation

> **New Features**: Check out [Canvas Navigation](canvas-navigation.md) for zoom and pan controls!

## Prerequisites

Before you begin, ensure you have the following installed:

- **Rust**: Install via [rustup](https://rustup.rs/).
- **Node.js & npm**: Required for Tailwind CSS processing.
- **cargo-leptos**: The build tool for Leptos applications.
  ```bash
  cargo install cargo-leptos
  ```
- **WASM Target**:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```

## Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/diagramix.git
   cd diagramix
   ```

2. Install NPM dependencies (for Tailwind CSS):
   ```bash
   npm install
   ```

## Development

To run the application in development mode with hot-reloading:

1. Start the Tailwind CSS watcher in one terminal:
   ```bash
   npm run watch:css
   ```

2. Start the Leptos development server in another terminal:
   ```bash
   cargo leptos watch
   ```

The application will be available at `http://127.0.0.1:3000`.

## Building for Production

To create an optimized production build:

1. Build the CSS:
   ```bash
   npm run build:css
   ```

2. Build the Rust application:
   ```bash
   cargo leptos build --release
   ```

The artifacts will be generated in `target/site`.