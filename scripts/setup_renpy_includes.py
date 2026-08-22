#!/usr/bin/env python3
"""
setup_renpy_includes.py

Automates setting up the Windows link libraries (python.def, libpython3.X.dll.a, libpython3.X.lib)
and pyo3-config profiles for whichever Ren'Py / Python version the user has installed.

Usage:
    python setup_renpy_includes.py <path_to_renpy_sdk_or_libpython_dll>
    
Examples:
    python setup_renpy_includes.py "C:/renpy-8.3.4-sdk"
    python setup_renpy_includes.py "C:/renpy-8.3.4-sdk/lib/py3-windows-x86_64/libpython3.12.dll"
    python setup_renpy_includes.py --detect (searches local renpy_includes directory)
"""

import os
import sys
import glob
import struct
import shutil
import subprocess
import re

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.dirname(SCRIPT_DIR)
INCLUDES_DIR = os.path.join(PROJECT_ROOT, "renpy_includes")
CONFIG_DIR = os.path.join(PROJECT_ROOT, "pyo3-config")

def find_dll(input_path):
    if os.path.isfile(input_path) and input_path.lower().endswith(".dll"):
        return input_path
    
    # If a directory was provided, look for libpython*.dll
    candidates = [
        os.path.join(input_path, "lib", "py3-windows-x86_64", "libpython*.dll"),
        os.path.join(input_path, "lib", "py3-windows-x86_64", "python*.dll"),
        os.path.join(input_path, "libpython*.dll"),
        os.path.join(input_path, "python*.dll"),
    ]
    for pattern in candidates:
        matches = glob.glob(pattern)
        if matches:
            return matches[0]
    
    # Check if a DLL already exists inside renpy_includes/
    local_matches = glob.glob(os.path.join(INCLUDES_DIR, "libpython*.dll")) + glob.glob(os.path.join(INCLUDES_DIR, "python*.dll"))
    if local_matches:
        return local_matches[0]

    return None

def extract_exports_and_dll_name(dll_path):
    with open(dll_path, 'rb') as f:
        data = f.read()

    e_lfanew = struct.unpack_from('<I', data, 0x3C)[0]
    opt_header_offset = e_lfanew + 24
    export_rva = struct.unpack_from('<I', data, opt_header_offset + 112)[0]
    num_sections = struct.unpack_from('<H', data, e_lfanew + 6)[0]
    sec_offset = e_lfanew + 24 + struct.unpack_from('<H', data, e_lfanew + 20)[0]

    def rva_to_offset(rva):
        for i in range(num_sections):
            sec = data[sec_offset + i*40 : sec_offset + (i+1)*40]
            name, v_size, v_addr, r_size, r_ptr = struct.unpack('<8sIIII', sec[:24])
            if v_addr <= rva < v_addr + max(v_size, r_size):
                return rva - v_addr + r_ptr
        return None

    exp_offset = rva_to_offset(export_rva)
    if not exp_offset:
        raise ValueError(f"Could not locate export directory in {dll_path}")

    # Extract internal DLL name
    name_rva = struct.unpack_from('<I', data, exp_offset + 12)[0]
    dll_name_offset = rva_to_offset(name_rva)
    internal_dll_name = data[dll_name_offset:].split(b'\0')[0].decode('ascii', errors='ignore')
    if not internal_dll_name:
        internal_dll_name = os.path.basename(dll_path)

    # Extract symbol names
    num_names = struct.unpack_from('<I', data, exp_offset + 24)[0]
    names_rva = struct.unpack_from('<I', data, exp_offset + 32)[0]
    names_offset = rva_to_offset(names_rva)

    exports = []
    for i in range(num_names):
        name_rva = struct.unpack_from('<I', data, names_offset + i * 4)[0]
        name_off = rva_to_offset(name_rva)
        exp_name = data[name_off:].split(b'\0')[0].decode('ascii', errors='ignore')
        exports.append(exp_name)

    return internal_dll_name, exports

def run_dlltool(def_path, dll_name, out_a_path, out_lib_path):
    # Try finding dlltool: zig dlltool -> dlltool -> llvm-dlltool
    cmd = None
    if shutil.which("zig"):
        cmd = ["zig", "dlltool"]
    elif shutil.which("dlltool"):
        cmd = ["dlltool"]
    elif shutil.which("llvm-dlltool"):
        cmd = ["llvm-dlltool"]
    
    if not cmd:
        print("[!] Warning: Neither 'zig', 'dlltool', nor 'llvm-dlltool' was found in PATH.")
        print("    Please install Zig (winget install zig.zig) or MinGW.")
        return False

    base_cmd = cmd + ["-m", "i386:x86-64", "-d", def_path, "-D", dll_name]
    
    print(f"[*] Running {' '.join(base_cmd)} -l {out_a_path}")
    subprocess.check_call(base_cmd + ["-l", out_a_path])
    
    if out_lib_path:
        subprocess.check_call(base_cmd + ["-l", out_lib_path])
    
    return True

def update_pyo3_configs(py_version_str, lib_name):
    """
    Updates the version=... and lib_name=... lines in pyo3-config files.
    """
    if not os.path.exists(CONFIG_DIR):
        return

    normalized_inc_dir = INCLUDES_DIR.replace("\\", "/")
    for cfg_file in os.listdir(CONFIG_DIR):
        if not cfg_file.endswith(".txt"):
            continue
        full_path = os.path.join(CONFIG_DIR, cfg_file)
        with open(full_path, 'r') as f:
            content = f.read()

        content = re.sub(r'(?m)^version=.*$', lambda m: f'version={py_version_str}', content)
        if cfg_file == "win.txt":
            content = re.sub(r'(?m)^lib_name=.*$', lambda m: f'lib_name={lib_name}', content)
            content = re.sub(r'(?m)^lib_dir=.*$', lambda m: 'lib_dir=renpy_includes', content)

        with open(full_path, 'w') as f:
            f.write(content)
        print(f"[+] Updated {cfg_file} to Python {py_version_str}")

def main():
    if len(sys.argv) < 2 or sys.argv[1] in ("-h", "--help"):
        input_path = INCLUDES_DIR
    else:
        input_path = sys.argv[1]

    os.makedirs(INCLUDES_DIR, exist_ok=True)
    dll_path = find_dll(input_path)

    if not dll_path:
        print(f"[x] Error: No libpython*.dll found in '{input_path}'.")
        print("    Please provide the path to your Ren'Py directory or libpython DLL.")
        sys.exit(1)

    print(f"[+] Found Ren'Py Python DLL: {dll_path}")

    # Copy DLL into renpy_includes if not already there
    dest_dll = os.path.join(INCLUDES_DIR, os.path.basename(dll_path))
    if os.path.abspath(dll_path) != os.path.abspath(dest_dll):
        shutil.copy2(dll_path, dest_dll)
        print(f"[+] Copied to {dest_dll}")

    dll_name, exports = extract_exports_and_dll_name(dest_dll)
    print(f"[+] Extracted {len(exports)} export symbols from {dll_name}")

    # Detect version from DLL name (e.g. libpython3.12.dll -> 3.12, python39.dll -> 3.9)
    match = re.search(r'python(\d)(\d+)', dll_name, re.IGNORECASE)
    if match:
        py_ver = f"{match.group(1)}.{match.group(2)}"
    else:
        match_dot = re.search(r'python(\d+\.\d+)', dll_name, re.IGNORECASE)
        py_ver = match_dot.group(1) if match_dot else "3.12"

    lib_base = os.path.splitext(dll_name)[0]

    # Write python.def
    def_path = os.path.join(INCLUDES_DIR, "python.def")
    with open(def_path, "w") as f:
        f.write("EXPORTS\n" + "\n".join(exports) + "\n")
    print(f"[+] Generated {def_path}")

    # Generate import libraries (.dll.a and .lib)
    out_a = os.path.join(INCLUDES_DIR, f"{lib_base}.dll.a")
    out_lib = os.path.join(INCLUDES_DIR, f"{lib_base}.lib")
    run_dlltool(def_path, dll_name, out_a, out_lib)

    # Also make a generic libpython3.X symlink/copy if needed
    update_pyo3_configs(py_ver, lib_base)

    print("\n[✓] Ren'Py includes and PyO3 configuration successfully set up!")
    print(f"    Python Version: {py_ver}")
    print(f"    Library Name:   {lib_base}")
    print(f"    Includes Path:  {INCLUDES_DIR}\n")

if __name__ == "__main__":
    main()
