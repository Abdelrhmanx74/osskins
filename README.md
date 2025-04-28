# Osskins

![App Screenshot](public/osskins-screenshot.png)

A cross‑platform desktop application built with Next.js, Tauri and TypeScript. Osskinns lets you browse, search and manage custom skins for League of Legends on Windows.

## Key Features

- **Tauri + Next.js**  
  Combines Tauri’s lightweight Rust backend and secure webview with Next.js App Router (v15) and Static Site Generation.
- **TypeScript & React**  
  Fully typed frontend with React 19, Zustand for state, and next-themes for dark/light mode.
- **TailwindCSS v4**  
  Utility‑first styling, customizable via Tailwind config.
- **Headless UI & Radix**  
  Accessible dialogs, menus, tooltips and more from @radix‑ui/react.
- **Modular Architecture**
  - `src/components` contains UI primitives and domain components.
  - `src/hooks` and `src/lib` for data fetching, local persistence and utilities.
  - `src-tauri/src` houses all Rust logic and Tauri commands.
- **Notifications & Feedback**  
  In-app toasts via Sonner, progress bars, status indicators and logs dialog.

## Prerequisites

- Node.js ≥18 & pnpm
- Rust toolchain (`rustup`, `cargo`)
- Windows 10+ (for Tauri’s native window)
