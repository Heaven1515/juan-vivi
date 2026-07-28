# -*- mode: python ; coding: utf-8 -*-
"""
Spec de PyInstaller para JUAN-VIVI.
Genera un instalador de un solo .exe con todo incluido:
  - Código Python + dependencias
  - dashboard.html
  - Tesseract OCR + eng.traineddata
  - Edge WebView2 (usa el del sistema — preinstalado en Windows 10/11)

Para compilar: python build.py
"""

import os
from pathlib import Path

TESSERACT_DIR = Path(r"C:\Program Files\Tesseract-OCR")

# ── Archivos de datos a incluir ───────────────────────────────────────────────
datas = [
    # Dashboard HTML
    ("dashboard.html", "."),
    # Tesseract binaries
    (str(TESSERACT_DIR / "tesseract.exe"),        "tesseract"),
    (str(TESSERACT_DIR / "tessdata" / "eng.traineddata"), "tesseract/tessdata"),
]

# DLLs de Tesseract (las que necesita tesseract.exe en Windows)
tess_dlls = [f for f in TESSERACT_DIR.glob("*.dll")]
binaries = [(str(dll), "tesseract") for dll in tess_dlls]

# ── Análisis ──────────────────────────────────────────────────────────────────
a = Analysis(
    ["dev.py"],
    pathex=["."],
    binaries=binaries,
    datas=datas,
    hiddenimports=[
        "webview",
        "webview.platforms.winforms",
        "PIL._tkinter_finder",
        "fitz",
        "pytesseract",
        "openpyxl",
        "requests",
        "clr",
    ],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["tkinter", "matplotlib", "numpy", "scipy"],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name="JUAN-VIVI",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    upx_exclude=[],
    runtime_tmpdir=None,
    console=False,          # sin ventana de consola
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=None,
)
