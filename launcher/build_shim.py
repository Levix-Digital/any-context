"""
Cross-Platform Build Script for AnyContext Native Launcher Shim.
Compiles actx_shim.cs on Windows using native csc.exe, or actx_shim.c on Linux using gcc/clang.
Produces an ultra-fast (< 2ms) native binary for instant version and flag routing.
"""
import os
import sys
import shutil
import subprocess
import argparse

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LAUNCHER_DIR = os.path.join(REPO_ROOT, "launcher")


def find_windows_csharp_compiler():
    """Locates the built-in Windows C# compiler (csc.exe)."""
    candidates = [
        r"C:\Windows\Microsoft.NET\Framework64\v4.0.30319\csc.exe",
        r"C:\Windows\Microsoft.NET\Framework\v4.0.30319\csc.exe",
    ]
    for p in candidates:
        if os.path.isfile(p):
            return p
    which_csc = shutil.which("csc")
    if which_csc:
        return which_csc
    return None


def build_windows_shim(out_path: str) -> bool:
    csc = find_windows_csharp_compiler()
    if not csc:
        print("❌ Error: Windows C# compiler (csc.exe) not found.")
        return False

    cs_file = os.path.join(LAUNCHER_DIR, "actx_shim.cs")
    cmd = [csc, "/nologo", "/optimize+", "/target:exe", f"/out:{out_path}", cs_file]
    print(f"🔨 Compiling Windows Launcher Shim: {' '.join(cmd)}")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"❌ csc compilation failed:\nSTDOUT: {res.stdout}\nSTDERR: {res.stderr}")
        return False
    print(f"✅ Successfully generated native Windows Launcher Shim: {out_path} ({os.path.getsize(out_path)} bytes)")
    return True


def build_linux_shim(out_path: str) -> bool:
    compiler = shutil.which("gcc") or shutil.which("clang") or shutil.which("cc")
    if not compiler:
        print("❌ Error: C compiler (gcc/clang) not found.")
        return False

    c_file = os.path.join(LAUNCHER_DIR, "actx_shim.c")
    cmd = [compiler, "-O3", c_file, "-o", out_path]
    print(f"🔨 Compiling Linux Launcher Shim: {' '.join(cmd)}")
    res = subprocess.run(cmd, capture_output=True, text=True)
    if res.returncode != 0:
        print(f"❌ C compilation failed:\nSTDOUT: {res.stdout}\nSTDERR: {res.stderr}")
        return False
    print(f"✅ Successfully generated native Linux Launcher Shim: {out_path} ({os.path.getsize(out_path)} bytes)")
    return True


def main():
    parser = argparse.ArgumentParser(description="Build AnyContext Native Launcher Shim")
    parser.add_argument("--out", type=str, default=None, help="Output file path for the binary")
    args = parser.parse_args()

    is_windows = sys.platform.startswith("win")
    default_name = "actx.exe" if is_windows else "actx"
    out_file = args.out or os.path.join(REPO_ROOT, "dist", default_name)

    os.makedirs(os.path.dirname(os.path.abspath(out_file)), exist_ok=True)

    success = build_windows_shim(out_file) if is_windows else build_linux_shim(out_file)
    if not success:
        sys.exit(1)


if __name__ == "__main__":
    main()
