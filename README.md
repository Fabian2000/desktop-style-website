# FabiScOS

A browser-based operating system. Not a website that looks like one - an actual OS with a kernel, system calls, a filesystem, and apps written in Python.

> **Early Release** - This is experimental. Expect bugs and rough edges.

## What is this?

FabiScOS is not a website styled to look like a desktop. It's a real operating system running in your browser.

The difference: There's an actual kernel controlling everything. Apps can't just do whatever they want - they have to ask the system for permission. Files aren't fake - they're stored persistently and survive browser restarts. When an app crashes, it doesn't take down the system - the OS catches the error and keeps running.

You get a desktop with draggable windows, a file manager, a terminal, and apps written in Python. But underneath, it's built the way real operating systems are built.

## Why is it an OS?

**It has a kernel.** Written in Rust, compiled to WebAssembly. It's the privileged layer between apps and the browser - apps have no direct access to storage, display, or network. They must go through the kernel, which checks permissions and controls what's allowed.

**It has system calls.** Apps communicate through defined interfaces. Want to read a file? System call. Show a button? System call. Close a window? System call. Over 170 functions that apps can use.

**It has a filesystem.** A real one with directories, files, permissions, symbolic links, and a trash bin. Unix-like operations - not a simulation.

**It has process isolation.** Each app runs in a sandbox. No access to your real computer. No access to other apps. No server connection - everything runs locally. If an app crashes, the system shows an in-app error and keeps running. The app dies, the OS lives.

**It has a window manager.** Real window state - position, size, z-index, minimize, focus. Drag windows, resize them, stack them. On mobile it switches to fullscreen automatically.

**It has memory management.** Apps need memory to remember things - what you typed, what you clicked, which mode is active. They request it from the kernel, use it while running, and the kernel cleans it up when the app closes. Each app gets its own isolated space.

## Security

The system protects itself from apps:

- Each app runs in its own isolated runtime - it can't see or touch other apps
- Each app renders in its own container - CSS tricks like `position: fixed` can't escape the window
- Each app has its own memory space - no access to another app's data
- System files are protected - apps can't modify or delete them
- If an app runs too long, the kernel kills it

## Tech Stack

- **Kernel:** Rust → WebAssembly
- **Apps:** Python (via RustPython interpreter)
- **UI:** Yew framework
- **Storage:** Browser-local, persistent

~21,000 lines of code.

## Installation

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [cargo-watch](https://crates.io/crates/cargo-watch) (for development)

### Development

```bash
cargo watch -x run -i "fabi-sc-code/pkg/*" -i "fabi-sc/resources/script/*"
```

This watches for changes, rebuilds automatically, and ignores generated files.

### Production Build

```bash
wasm-pack build fabi-sc-code --target web --release
```

## How AI Was Used

This project was built by a human. AI assisted with parts of it.

The entire foundation, the architecture, major features - written by hand. AI was brought in for specific tasks where it made sense, not as a replacement for actually writing code.

When AI was used, every suggestion went through manual review. No auto-accept, no blind trust. Some harmless operations like compilation were pre-approved, but code changes always required explicit okay.

AI can be useful when controlled properly. This project is an example of that - using it as one tool among many, not letting it run wild, and only accepting what actually makes sense.
