# Architecture & Tech Stack

## Overview

Diagramix is a full-stack web application built with Rust, utilizing Server-Side Rendering (SSR) with client-side hydration. This architecture ensures fast initial page loads and SEO friendliness while maintaining the interactivity of a Single Page Application (SPA).

## Tech Stack

### Core Framework
- **[Leptos](https://leptos.dev/)**: A reactive web framework for Rust. We use Leptos for both the frontend (WASM) and the backend (SSR).
- **[Axum](https://github.com/tokio-rs/axum)**: The backend server framework that hosts the Leptos application.
- **[Tokio](https://tokio.rs/)**: The asynchronous runtime powering the backend.

### Frontend & Styling
- **WebAssembly (WASM)**: The client-side logic is compiled to WASM for high performance.
- **[Tailwind CSS](https://tailwindcss.com/)**: A utility-first CSS framework used for styling components.
- **[Leptos Meta](https://docs.rs/leptos_meta)**: Manages metadata (title, meta tags) for the application.

### Data Structures
- **[Petgraph](https://github.com/petgraph/petgraph)**: A graph data structure library used to represent and manipulate the database schema diagrams. It handles the underlying node and edge relationships.
- **[Serde](https://serde.rs/)**: Used for serializing and deserializing data structures, essential for passing data between the server and client.

## Key Concepts

### SSR & Hydration
The application starts on the server (`ssr` feature). When a user requests a page, the server renders the HTML and sends it to the browser. Once the WASM bundle loads, the application "hydrates," attaching event listeners and taking over state management on the client side (`hydrate` feature).

### Diagram Logic
The core business logic revolves around visualizing database schemas. `petgraph` is used to model tables as nodes and relationships (foreign keys) as edges. This allows for efficient traversal and layout calculations.