"""
build.py — Compila JUAN-VIVI completo.

Pasos:
  1. PyInstaller -> sidecar Python (.exe)
  2. Copia sidecar a src-tauri/binaries/
  3. cargo tauri build -> instalador NSIS

Requisitos previos:
  - Iconos: debes proveer src-tauri/icons/ con los siguientes archivos:
      32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico, icon.png
    Sin estos archivos, `cargo tauri build` fallara.
    Puedes generarlos desde un PNG con: npm run tauri icon <ruta_a_icono.png>

Uso:
    python build.py
"""
import subprocess
import sys
import shutil
from pathlib import Path

TESSERACT_DIR = Path(r"C:\Program Files\Tesseract-OCR")
RAIZ = Path(__file__).parent
SIDECAR_NAME = "juan-vivi-backend-x86_64-pc-windows-msvc"


def verificar():
    errores = []
    if not TESSERACT_DIR.exists():
        errores.append(f"Tesseract no encontrado en {TESSERACT_DIR}")
    try:
        import PyInstaller
    except ImportError:
        errores.append("PyInstaller no instalado -- pip install pyinstaller")
    if shutil.which("cargo") is None:
        errores.append("Cargo no encontrado -- instalar Rust")

    # Verificar iconos
    icons_dir = RAIZ / "src-tauri" / "icons"
    required_icons = ["32x32.png", "128x128.png", "128x128@2x.png", "icon.icns", "icon.ico", "icon.png"]
    missing_icons = [i for i in required_icons if not (icons_dir / i).exists()]
    if missing_icons:
        errores.append(
            f"Faltan iconos en src-tauri/icons/: {', '.join(missing_icons)}\n"
            "  Genera con: npm run tauri icon <tu_logo.png>"
        )

    if errores:
        print("\n[BUILD] Errores:\n")
        for e in errores:
            print(f"  x {e}")
        sys.exit(1)
    print("[BUILD] Requisitos OK")


def compilar_sidecar():
    print("\n[BUILD] Compilando sidecar Python...")
    r = subprocess.run(
        [sys.executable, "-m", "PyInstaller", "server.spec", "--clean", "--noconfirm"],
        cwd=RAIZ,
    )
    if r.returncode != 0:
        print("[BUILD] Error compilando sidecar")
        sys.exit(1)

    # Copiar a src-tauri/binaries/
    src = RAIZ / "dist" / f"{SIDECAR_NAME}.exe"
    dst_dir = RAIZ / "src-tauri" / "binaries"
    dst_dir.mkdir(exist_ok=True)
    shutil.copy2(src, dst_dir / f"{SIDECAR_NAME}.exe")
    print(f"[BUILD] Sidecar copiado -> src-tauri/binaries/{SIDECAR_NAME}.exe")


def compilar_tauri():
    print("\n[BUILD] Compilando Tauri...")
    r = subprocess.run(
        ["cargo", "tauri", "build"],
        cwd=RAIZ,
    )
    if r.returncode != 0:
        print("[BUILD] Error compilando Tauri")
        sys.exit(1)


def informar():
    installer = next((RAIZ / "src-tauri" / "target" / "release" / "bundle" / "nsis").glob("*.exe"), None)
    if installer:
        size_mb = installer.stat().st_size / 1_048_576
        print(f"\n[BUILD] Instalador listo: {installer}")
        print(f"        Tamano: {size_mb:.0f} MB")
        print("\n  Pasos para instalar en el PC de oficina:")
        print("  1. Copiar el instalador al PC de oficina")
        print("  2. Ejecutar el instalador (doble clic)")
        print("  3. Futuras actualizaciones: automaticas desde GitHub Releases")
    else:
        print("[BUILD] No se encontro el instalador NSIS")


if __name__ == "__main__":
    verificar()
    compilar_sidecar()
    compilar_tauri()
    informar()
