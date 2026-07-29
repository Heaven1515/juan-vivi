# JUAN-VIVI — Estado del Proyecto
> Actualizado: 2026-07-29 | Arquitectura: Rust puro (migración completa)

---

## Qué es este proyecto

**JUAN-VIVI** es una app de escritorio para la **33ª Notaría** que hace dos cosas:
1. **Módulo Repertorios**: procesa planillas Excel AUTOFIN SP y genera planilla masivos para el SRCeI
2. **Módulo Firma Electrónica**: gestiona el envío de PDFs escaneados a SIGN+ (manual y automático con OCR)

---

## Arquitectura — Rust puro (migración completa ✅)

```
Frontend (JS puro)
    ↓ invoke()
Tauri Commands (Rust puro)
  — sin sidecar, sin HTTP, sin PyInstaller —
```

**Por qué se migró:** PyInstaller extrae DLLs en `%TEMP%` en cada arranque → el antivirus corporativo las mata → crash. Con Rust nativo no hay extracción, no hay proceso separado, el antivirus escanea una vez al instalar.

---

## Módulos Rust — todos completos ✅

| Módulo | Archivo | Comandos Tauri |
|---|---|---|
| Licencia / kill switch | `licencia.rs` | `verificar_licencia` |
| BD casos de firma (JSON) | `casos.rs` | `buscar_caso`, `agregar_caso`, `listar_casos`, `eliminar_caso` |
| BD repertorios históricos (JSON) | `repertorios.rs` | `buscar_repertorio`, `reemplazar_repertorio`, `cargar_repertorios_excel` |
| Leer Excel AUTOFIN SP | `lector.rs` | — (interno) |
| Transformador lógica pura | `transformador.rs` | — (interno) |
| Escribir planilla masivos | `escritor.rs` | — (interno) |
| Comandos planilla completos | `planilla_cmd.rs` | `cargar_excel`, `nombre_planilla`, `generar_planilla` |
| OCR (PDF → texto) | `ocr.rs` | — (interno, usado por firma) |
| Envío a SIGN+ | `signplus.rs` | — (interno, usado por firma) |
| Vigilador carpeta escáner | `vigilador.rs` | — (interno, Tokio polling) |
| Orquestador Firma | `firma.rs` | `get_carpeta_scanner`, `set_carpeta_scanner`, `listar_pdfs`, `pdf_pagina`, `enviar_pdf`, `toggle_auto`, `estado_auto`, `get_log_firma` |

---

## Estructura de archivos — Rust

```
src-tauri/src/
├── main.rs              ← punto de entrada
├── lib.rs               ← Tauri builder, todos los comandos registrados
├── almacenamiento.rs    ← dir_datos(): dev=data/, release=%LOCALAPPDATA%\JUAN-VIVI
├── modelos.rs           ← structs: Caso, Registro, FilaEntrada, FilaSalida, EstadoFirma, etc.
├── casos.rs             ← CRUD casos.json
├── licencia.rs          ← verificar_licencia() async via GitHub API
├── repertorios.rs       ← CRUD repertorios.json + cargar desde Excel
├── lector.rs            ← leer Excel AUTOFIN SP (calamine)
├── transformador.rs     ← lógica pura AUTOFIN (sin deps externas)
├── escritor.rs          ← escribir planilla masivos (rust_xlsxwriter)
├── planilla_cmd.rs      ← comandos Tauri para cargar/generar planilla
├── signplus.rs          ← cliente HTTP SIGN+ (reqwest multipart + JWT)
├── vigilador.rs         ← watcher carpeta escáner (Tokio polling, UNC compatible)
├── ocr.rs               ← pdfium-render + tesseract subprocess, extraer_datos + renderizar_pagina
└── firma.rs             ← orquestador módulo Firma, Arc<Mutex<EstadoFirma>>

src-tauri/resources/
├── pdfium.dll           ← descargado de bblanchon/pdfium-binaries en CI
└── tesseract/
    ├── tesseract.exe
    └── tessdata/
        └── eng.traineddata
```

---

## Crates Rust usados

| Crate | Propósito |
|---|---|
| `serde` / `serde_json` | Serialización JSON |
| `calamine` | Leer Excel (.xlsx) |
| `rust_xlsxwriter` | Escribir Excel con formato |
| `reqwest` | HTTP (licencia + SIGN+) con multipart y cookies |
| `regex` | Expresiones regulares |
| `chrono` | Fechas (serial Excel → YYYY-MM-DD) |
| `base64` | Decodificar respuesta GitHub API |
| `dirs` | Rutas de AppData por plataforma |
| `anyhow` | Manejo de errores interno |
| `tokio` | Runtime async (full features) |
| `pdfium-render` | Renderizado PDF (viewer y OCR) — con image + thread_safe |
| `image` | Guardar frames pdfium como PNG para tesseract |
| `once_cell` | Lazy static para regex patterns en OCR |

---

## Reglas de negocio del módulo Repertorios

| Col K | Comportamiento |
|---|---|
| vacío (normal) | Compareciente 2 = NOMBRE / RUT (limpia \xa0) |
| `COMPRA PARA` | Compareciente 2 = NOMBRE PARA / RUT COMPRA PARA |
| `COMUNIDAD` | Compareciente 2 = primer nombre + " Y OTRO" / primer RUT (antes del /) |
| Fila vacía (sin nombre ni RUT) | Aviso y omisión silenciosa |

Compareciente 1 SIEMPRE: `76139506-8` / `AUTOFIN S.A.`

---

## Módulo Firma Electrónica

- **Carpeta escáner**: `\\Desktop-14rq1mp\escaner riho 550` (configurable en runtime via `set_carpeta_scanner`)
- **Modo manual**: usuario selecciona PDF → ve páginas → llena formulario → envía a SIGN+
- **Modo automático**: vigilador detecta PDF nuevo → OCR → busca en BD casos/repertorios → envía solo
- **OCR**: pdfium-render 2.5x scale → PNG temp → tesseract subprocess (eng) — 100% Rust/subprocess nativo
- **SIGN+**: `http://192.168.1.177` — solo red interna notaría
  - Usuario: `JESPINA` / Password: `Cpina2026`
  - ⚠️ Pendiente: mover a Windows Credential Manager
- **Vigilador**: Tokio polling cada 8s (UNC paths compatibles), espera 12s antes de callback

---

## Kill switch

Editar `licencia.json` en GitHub: `activo: false` + mensaje. Efectivo inmediato via GitHub API (sin cache CDN). Para restaurar: `activo: true`. Verificación directamente desde Rust.

---

## Versiones publicadas

| Versión | Cambios |
|---|---|
| v1.0.0 | Primera versión — Tauri + sidecar FastAPI |
| v1.0.1 | Kill switch via licencia.json + fix initApi |
| v1.0.2 | Fix polling sidecar antes de checkLicencia |
| v1.0.3 | Fix CDN cache — GitHub API en vez de raw |
| v1.0.4 | Fix procesos zombie — child.kill() al cerrar |
| v1.0.5 | BD repertorios históricos + buscador + modal duplicados |
| v1.0.6 | Auto-updater con latest.json manual en GitHub Actions |
| v1.0.7 | Fix join timeout vigilador + buscador z-index + _buscar_datos fallback |
| v1.0.8 | Remove casos hardcodeados + botón verde BD + X modal duplicados |
| v1.0.9 | Auto-restart backend + banner reconexión + /ping healthcheck |
| v2.0.0 | Migración Fase 1: planilla + casos + repertorios + licencia → Rust puro |
| **v2.1.0** | **Migración Fase 2: módulo Firma completo en Rust — sin sidecar Python** |

---

## Para publicar una nueva versión

```bash
git tag vX.Y.Z && git push origin vX.Y.Z
```

GitHub Actions compila en ~10 min. **Solo `cargo tauri build`** — sin pasos Python ni PyInstaller.

Variable de entorno requerida en GitHub Secrets: `TAURI_SIGNING_PRIVATE_KEY`
Llave privada local: `.tauri-key` — backup en OneDrive, NO regenerar.

---

## Configuración SIGN+

```
URL_BASE  = http://192.168.1.177
URL_API   = http://192.168.1.177/app/escrituras_publicas/api/
USUARIO   = JESPINA
PASSWORD  = Cpina2026
```

⚠️ Credenciales en signplus.rs — mover a Windows Credential Manager en próxima versión.

---

## Próximos pasos opcionales

1. Mover credenciales SIGN+ a Windows Credential Manager (keychain)
2. Probar en PC de oficina — verificar que antivirus no bloquea recursos bundleados
3. Ajustar OCR si regex patterns necesitan calibración con PDFs reales de la notaría
