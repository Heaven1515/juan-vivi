"""
build.py — Empaqueta JUAN-VIVI a un .exe autónomo.

Uso:
    python build.py

El ejecutable queda en dist/JUAN-VIVI.exe
Copiarlo al PC de oficina y hacer doble clic — no requiere instalar nada.
"""

import subprocess
import sys
import os
from pathlib import Path

TESSERACT_DIR = Path(r"C:\Program Files\Tesseract-OCR")


def verificar_requisitos():
    errores = []

    if not TESSERACT_DIR.exists():
        errores.append(
            f"Tesseract no encontrado en {TESSERACT_DIR}\n"
            "  Instalar desde: https://github.com/UB-Mannheim/tesseract/wiki"
        )

    if not (TESSERACT_DIR / "tessdata" / "eng.traineddata").exists():
        errores.append("eng.traineddata no encontrado en tessdata/")

    try:
        import PyInstaller
    except ImportError:
        errores.append("PyInstaller no instalado — ejecutar: pip install pyinstaller")

    if errores:
        print("\n[BUILD] Errores encontrados:\n")
        for e in errores:
            print(f"  ✗ {e}\n")
        sys.exit(1)

    print("[BUILD] Requisitos OK")


def compilar():
    print("[BUILD] Compilando JUAN-VIVI.exe ...\n")
    resultado = subprocess.run(
        [sys.executable, "-m", "PyInstaller", "juan_vivi.spec", "--clean", "--noconfirm"],
        cwd=Path(__file__).parent,
    )
    if resultado.returncode != 0:
        print("\n[BUILD] Error durante la compilación.")
        sys.exit(1)


def informar():
    exe = Path("dist") / "JUAN-VIVI.exe"
    if exe.exists():
        size_mb = exe.stat().st_size / 1_048_576
        print(f"\n[BUILD] Listo: {exe.resolve()}")
        print(f"        Tamaño: {size_mb:.0f} MB")
        print("\n  Para instalar en el PC de oficina:")
        print("  1. Copiar JUAN-VIVI.exe al escritorio (o donde se quiera)")
        print("  2. Doble clic — no requiere instalar nada")
    else:
        print("[BUILD] No se encontró el ejecutable en dist/")


if __name__ == "__main__":
    verificar_requisitos()
    compilar()
    informar()
