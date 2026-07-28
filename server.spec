# -*- mode: python ; coding: utf-8 -*-
"""
PyInstaller spec para el sidecar Python de JUAN-VIVI.
Genera: binaries/juan-vivi-backend-x86_64-pc-windows-msvc.exe
"""
from pathlib import Path

TESSERACT_DIR = Path(r"C:\Program Files\Tesseract-OCR")

datas = [
    (str(TESSERACT_DIR / "tesseract.exe"),                      "tesseract"),
    (str(TESSERACT_DIR / "tessdata" / "eng.traineddata"),       "tesseract/tessdata"),
]

binaries = [(str(dll), "tesseract") for dll in TESSERACT_DIR.glob("*.dll")]

a = Analysis(
    ["server.py"],
    pathex=["."],
    binaries=binaries,
    datas=datas,
    hiddenimports=[
        "uvicorn.logging",
        "uvicorn.loops",
        "uvicorn.loops.auto",
        "uvicorn.protocols",
        "uvicorn.protocols.http",
        "uvicorn.protocols.http.auto",
        "uvicorn.protocols.websockets",
        "uvicorn.protocols.websockets.auto",
        "uvicorn.lifespan",
        "uvicorn.lifespan.on",
        "fastapi",
        "fitz",
        "pytesseract",
        "openpyxl",
        "requests",
        "PIL",
    ],
    excludes=["tkinter", "matplotlib", "numpy"],
    noarchive=False,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    a.binaries,
    a.datas,
    [],
    name="juan-vivi-backend-x86_64-pc-windows-msvc",
    debug=False,
    console=True,    # consola activa: sidecar, no ventana de usuario
    upx=True,
)
