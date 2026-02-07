# Mehu - PLOT3D Viewer

A modern, cross-platform application for visualizing CFD (Computational Fluid Dynamics) grid and solution data in PLOT3D format.

> **About the Name**: "Mehu" is a Twi phrase (a language spoken in Ghana) meaning "I see" — reflecting the core purpose of this visualization tool.

## Features

- **PLOT3D File Support**: Read and parse PLOT3D binary grid files
- **3D Visualization**: Interactive 3D rendering using Three.js
- **Wireframe & Shaded Modes**: Toggle between wireframe and flat-shaded rendering
- **Multi-Grid Support**: Handle multiple computational grids
- **Cross-Platform**: Runs on Linux, Windows, and macOS

## Tech Stack

- **Frontend**: React + TypeScript + Three.js
- **Backend**: Rust (via Tauri)
- **3D Rendering**: React Three Fiber + Drei
- **Desktop Framework**: Tauri 2.0

## Prerequisites

- Node.js (v20 or later)
- Rust (latest stable)
- npm

### System Requirements for Pre-built Binaries

**macOS**: 
- macOS 11.0 or later
- Intel (x86_64) or Apple Silicon (aarch64)

**Linux**:
- glibc 2.35 or later (Ubuntu 22.04+, Fedora 36+, Debian 12+, Rocky Linux 9+)
- AppImage format (no installation required, just make executable and run)

**Windows**:
- Windows 10 or later
- MSI installer

## Getting Started

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Project Structure

```
mehu/
├── src/                    # Frontend React/TypeScript code
│   ├── components/         # React components
│   │   └── Viewer3D.tsx   # Main 3D viewer component
│   ├── App.tsx            # Main app component
│   └── main.tsx           # Entry point
├── src-tauri/             # Rust backend code
│   ├── src/
│   │   ├── lib.rs         # Main Tauri application
│   │   └── plot3d.rs      # PLOT3D file parser
│   └── Cargo.toml         # Rust dependencies
└── package.json           # Node.js dependencies
```

## Architecture

**Frontend (React + Three.js)**:
- Handles UI and 3D visualization
- Lightweight, focuses on rendering only
- Uses React Three Fiber for declarative 3D scenes

**Backend (Rust)**:
- Parses PLOT3D binary files efficiently
- Manages large mesh data (million+ points)
- Provides Tauri commands for file operations

## Building for Distribution

Automated builds are handled via GitHub Actions. Binaries are built for:
- **macOS**: Both Intel and Apple Silicon architectures in one .app bundle
- **Linux**: AppImage (no installation needed, portable executable)
- **Windows**: MSI installer

Builds run automatically on push to `main` branch and tagged releases. Artifacts are available in the GitHub Actions tab.

PLOT3D is a NASA-developed format for storing CFD grid and solution data. For more information, see the [PLOT3D manual](https://ntrs.nasa.gov/api/citations/19900013774/downloads/19900013774.pdf).

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
