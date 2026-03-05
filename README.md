# Depot Downloader (Rust)

A native Rust GUI application for downloading Steam game content using depot keys and manifests.

## Features

- **Native Rust GUI** built with egui
- **Cross-platform** support (Windows, Linux, macOS)
- **Steam Web API** integration for game info lookup
- **ManifestHub repository** support for depot keys
- **Manifest decryption** using AES-128-CTR
- **Parallel chunk-based downloading** with progress tracking
- **Settings management** with persistence
- **Logging** with GUI integration

## Project Structure

```
depot_downloader/
├── Cargo.toml          # Project dependencies
├── src/
│   ├── main.rs         # Application entry point
│   ├── steam/          # Steam integration
│   │   ├── mod.rs
│   │   ├── web_api.rs  # Steam Web API client
│   │   ├── auth.rs     # Steam authentication
│   │   └── session.rs  # Steam session management
│   ├── cdn/            # CDN download functionality
│   │   ├── mod.rs
│   │   ├── client.rs   # CDN client
│   │   ├── server.rs   # CDN server management
│   │   └── download.rs # Download manager
│   ├── manifest/       # Manifest handling
│   │   ├── mod.rs
│   │   ├── parser.rs   # Manifest parsing
│   │   ├── decryption.rs # Manifest decryption
│   │   └── store.rs    # Manifest storage
│   ├── gui/            # GUI components
│   │   ├── mod.rs
│   │   ├── app.rs      # Main application
│   │   ├── components.rs # UI components
│   │   └── dialogs.rs  # Modal dialogs
│   └── config/         # Configuration
│       ├── mod.rs
│       ├── settings.rs # App settings
│       └── credentials.rs # Steam credentials
```

## Building

### Prerequisites

- Rust 1.70+ (2021 edition)
- System dependencies:
  - Linux: `libssl-dev`, `pkg-config`
  - Windows: Visual Studio or MinGW
  - macOS: Xcode Command Line Tools

### Build

```bash
cd depot_downloader
cargo build --release
```

### Run

```bash
cargo run
```

## Usage

1. Launch the application
2. Enter a Steam App ID in the search box
3. Click "Search" to fetch game information
4. Click "Get Depot Keys" to fetch keys from ManifestHub
5. Click "Download" to configure and start the download

## Configuration

Settings are stored in:
- Linux: `~/.config/depot_downloader/`
- Windows: `%APPDATA%\depot_downloader\`
- macOS: `~/Library/Application Support/depot_downloader/`

## Dependencies

- **egui/eframe**: Native Rust GUI
- **tokio**: Async runtime
- **reqwest**: HTTP client
- **serde/serde_json**: Serialization
- **prost**: Protobuf support
- **aes/ctr**: Cryptography
- **chrono**: Date/time handling
- **anyhow**: Error handling

## Architecture

This application is a port of the C# DepotDownloaderGUI, rewritten in Rust with the following improvements:

1. **Native GUI**: Uses egui instead of Windows Forms
2. **Cross-platform**: Runs on Windows, Linux, and macOS
3. **Modern async**: Uses tokio for async operations
4. **Type safety**: Leverages Rust's type system
5. **Performance**: Zero-cost abstractions

## License

GPL-2.0 license (matching the original project)

## Credits

- Original C# project: https://github.com/64dd/depotdownloadergui
- Based on SteamKit2
