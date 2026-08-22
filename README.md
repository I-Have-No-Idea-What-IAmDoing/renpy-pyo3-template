# Ren'Py PyO3 Rust Template

A template for Rust C-extension modules for **Ren'Py 8** using **PyO3**.

Supports cross-compilation from Windows, Linux, and macOS for:
* **Windows (x86_64)**
* **Linux (x86_64 & AArch64/ARM64)**
* **macOS (Apple Silicon & Intel x86_64)**

Includes Dev Containers and GitHub Actions CI/CD

---
## Requirements for Host machine:
* Rust & Cargo (duh...)
* cargo-zigbuild
  * Zig (Used for cross-platform building)
* Python 3 (Setup scripts)

---

## Usage
```bash
# Run this first to get the import library for your specific Renpy version
# The directory should point to the root folder of the SDK (The one that contains 'renpy.exe')
# This also works for compiled renpy games (since renpy itself is a renpy game)
cargo xtask setup --renpy-dir "C:/path/to/renpy-8.3.4-sdk"

# Build for all targets
cargo xtask dist

# Build specific targets
cargo xtask build --targets windows
cargo xtask dist --targets linux,mac

# Run Rust and Python test suite
cargo xtask test

# Watch mode (automatically rebuilds and deploys to your Ren'Py game on file save)
cargo xtask watch
cargo xtask watch --deploy-to-game "C:/path/to/my_renpy_game"

# Clean target and dist directories
cargo xtask clean
```

*Or use the shell script wrappers:*
* **Windows**: `.\build.ps1`
* **Linux / macOS**: `./build.sh`

---

## Declarative Configuration (`renpy-plugin.toml`)

You can configure your build, targets, and automatic game folder deployment directly in `renpy-plugin.toml`:

```toml
[plugin]
name = "my_custom_module"
version = "0.1.0"

[renpy]
python_version = "3.12"

[build]
targets = ["windows", "linux-x86_64", "linux-aarch64", "mac-arm64", "mac-x86_64"]
dist_dir = "dist"

# Optional: Absolute path to your Ren'Py game project to automatically copy binaries into
# deploy_to_game = "/path/to/my_renpy_game"
deploy_to_game = ""
```

---

## How to Rename the Module

1. In `Cargo.toml` and `renpy-plugin.toml`, set `name = "my_custom_module"`.
2. In `src/lib.rs`, rename the module entry function:
   ```rust
   #[pymodule]
   fn my_custom_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
       m.add_function(wrap_pyfunction!(greet, m)?)?;
       Ok(())
   }
   ```
3. Run `.\build.ps1` (or `./build.sh`).

---

## How to Distribute to Ren'Py

### Option A: `game/python-packages/` (Recommended for self-contained plugins)
Copy the contents of `dist/python-packages/` into your Ren'Py project's `game/python-packages/`:
```text
game/python-packages/
├── my_custom_module.cp312-win_amd64.pyd
├── my_custom_module.cpython-312-x86_64-linux-gnu.so
├── my_custom_module.cpython-312-aarch64-linux-gnu.so
├── my_custom_module.cpython-312-darwin.so
└── my_custom_module.cpython-312-darwin-x86_64.so
```

### Option B: Engine `lib/` directory
Copy the contents of `dist/lib/` into your Ren'Py game's root `lib/` directory:
```text
lib/
├── py3-windows-x86_64/ (my_custom_module.pyd)
├── py3-linux-x86_64/   (my_custom_module.so)
├── py3-linux-aarch64/  (my_custom_module.so)
├── py3-mac-arm64/      (my_custom_module.so)
└── py3-mac-x86_64/     (my_custom_module.so)
```

---

## Ren'Py Usage Example

In any `.rpy` script (e.g. `game/script.rpy`):

```python
init python:
    import my_custom_module

    # Call a standalone Rust function
    msg = my_custom_module.greet("Player")

    # Create and use a Struct from Rust
    tracker = my_custom_module.GameStateTracker(starting_points=100)
    score = tracker.add_points(50)

label start:
    "[msg]"
    "Score: [score]"
    return
```
