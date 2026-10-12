# Desktop app

`apps/desktop` (binary `rust-app-desktop`) is the composition root for the native GUI:

- eframe 0.36 with the **wgpu** renderer (Metal on macOS, DX12/Vulkan on Windows, Vulkan on Linux),
  accesskit, X11 and Wayland.
- `SqliteItemRepository` in the per-user data directory (`--data-dir`, `--in-memory`).
- `ThreadJobRunner` with `cores - 1` (max 4) workers; workers call `egui::Context::request_repaint`
  after every job update, so progress appears without polling.
- `Shell::logic` runs in `eframe::App::logic` (job polling, events), `Shell::ui` draws.

```bash
cargo xtask desktop                     # debug
cargo xtask desktop -- --in-memory      # nothing written to disk
cargo build --release -p starter-desktop
```

Start-up errors (e.g. an unreadable database, a database from a newer version) are reported in the
terminal with exit code 1 before a window opens.

Windows release builds use the GUI subsystem (no console window).

## Packaging (not included)

Installers are out of scope for the starter. Options when a project needs them: `cargo-packager` or
`cargo-dist` (dmg/msi/AppImage/deb), plus code signing (Apple notarisation, Windows Authenticode).
