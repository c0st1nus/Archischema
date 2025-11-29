# Diagramix

**Diagramix** is a modern, high-performance Database Schema Diagram Editor built with Rust and WebAssembly. It allows users to visualize and design database schemas with an intuitive interface.

## 🚀 Features

- **Full-Stack Rust**: Built with [Leptos](https://leptos.dev/) for a seamless SSR and hydration experience.
- **High Performance**: Powered by WebAssembly and [Petgraph](https://github.com/petgraph/petgraph) for efficient graph manipulation.
- **Modern UI**: Styled with [Tailwind CSS](https://tailwindcss.com/) for a clean and responsive design.
- **Interactive Canvas**: 
  - 🔍 **Zoom**: Use `Ctrl + Scroll` or `Ctrl + +/-` to zoom in and out
  - 🖱️ **Pan**: Hold middle mouse button to pan around the canvas
  - ✋ **Drag & Drop**: Click and drag tables to reposition them

## 📚 Documentation

For detailed instructions and architectural overview, please refer to the documentation folder:

- [**Setup & Installation**](docs/setup.md): How to build, run, and deploy the project.
- [**Architecture**](docs/architecture.md): Deep dive into the tech stack and design decisions.

## ⚡ Quick Start

If you have `cargo-leptos` and `npm` installed:

```bash
npm install
npm run watch:css &
cargo leptos watch
```

Visit `http://127.0.0.1:3000` to see the app in action.
