# Ren'Py Includes Directory

This directory holds the Windows linking definitions (python.def) and import libraries (.dll.a, .lib) corresponding to your installed Ren'Py version.

---

## How to Set Up for Your Ren'Py Version

### Method 1: Automatic Setup (Recommended)
Run the setup script pointing to your Ren'Py SDK folder:
`powershell
python scripts/setup_renpy_includes.py "C:/path/to/renpy-8.3.4-sdk"
`
Or pass it directly when building:
`powershell
.\build.ps1 -RenpyDir "C:/path/to/renpy-8.3.4-sdk"
`

The script will automatically:
1. Locate libpython*.dll in your Ren'Py SDK.
2. Extract the export symbol table to python.def.
3. Generate the import libraries (libpython3.X.dll.a and libpython3.X.lib) using zig dlltool.
4. Update pyo3-config/ to match your Ren'Py Python version (e.g. 3.9, 3.12, 3.13).

---

### Method 2: Manual Copy
1. Go to <YourRenpySDK>/lib/py3-windows-x86_64/
2. Copy libpython3.*.dll into this enpy_includes/ directory.
3. Run:
   ```powershell
   python scripts/setup_renpy_includes.py --detect
   ```
```
