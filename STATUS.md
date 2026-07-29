# JUAN-VIVI — Estado del Proyecto
> Actualizado: 2026-07-29 | Arquitectura: Migración a Rust puro en curso

---

## Qué es este proyecto

**JUAN-VIVI** es una app de escritorio para la **33ª Notaría** que hace dos cosas:
1. **Módulo Repertorios**: procesa planillas Excel AUTOFIN SP y genera planilla masivos para el SRCeI
2. **Módulo Firma Electrónica**: gestiona el envío de PDFs escaneados a SIGN+ (manual y automático)

---

## Arquitectura — En migración a Rust puro

### Estado actual (híbrido de transición)

```
Frontend (JS puro)
    ↓ invoke()        ↓ apiFetch() (solo firma)
Tauri Commands   Python FastAPI sidecar
  (Rust puro)       (solo módulo Firma)
```

### Arquitectura objetivo (100% Rust)

```
Frontend (JS puro)
    ↓ invoke()
Tauri Commands (Rust puro)
  — sin sidecar, sin HTTP, sin PyInstaller —
```

**Por qué migrar:** PyInstaller extrae DLLs en `%TEMP%` en cada arranque → el antivirus corporativo las mata → crash. La solución real es que todo el código sea nativo Rust: no hay extracción, no hay proceso separado, el antivirus escanea una vez al instalar.

---

## Estado de la migración

### Módulos migrados a Rust Tauri commands ✅

| Módulo | Rust | Frontend |
|---|---|---|
| Licencia / kill switch | `licencia.rs` | `invoke('verificar_licencia')` |
| BD casos de firma (JSON) | `casos.rs` | `invoke('buscar_caso / agregar_caso / listar_casos')` |
| BD repertorios históricos (JSON) | `repertorios.rs` | `invoke('buscar_repertorio / cargar_repertorios_excel / reemplazar_repertorio')` |
| Leer Excel AUTOFIN SP | `lector.rs` | — |
| Transformador (lógica pura) | `transformador.rs` | — |
| Escribir planilla masivos | `escritor.rs` | — |
| Comandos planilla completos | `planilla_cmd.rs` | `invoke('cargar_excel / nombre_planilla / generar_planilla')` |

### Módulos pendientes de migrar a Rust ⏳

| Módulo | Archivo Python actual | Crates Rust necesarios |
|---|---|---|
| OCR (PDF → texto) | `ocr_lector.py` (PyMuPDF + Tesseract) | `pdfium-render` + tesseract subprocess |
| Envío a SIGN+ | `signplus.py` (requests + multipart) | `reqwest` (ya disponible) |
| Vigilador de carpeta | `vigilador.py` (polling) | `notify` |
| Orquestador Firma | `firma_controller.py` | Rust puro |
| Visor PDF (páginas) | endpoint en `server.py` (fitz) | `pdfium-render` |

Mientras el módulo Firma no esté migrado, **el sidecar Python sigue activo** solo para esos endpoints.

---

## Estructura de archivos — Rust

```
src-tauri/src/
├── main.rs              ← punto de entrada
├── lib.rs               ← Tauri builder, todos los comandos registrados
├── almacenamiento.rs    ← dir_datos(): dev=data/, release=%LOCALAPPDATA%\JUAN-VIVI
├── modelos.rs           ← structs: Caso, Registro, FilaEntrada, FilaSalida, etc.
├── casos.rs             ← CRUD casos.json
├── licencia.rs          ← verificar_licencia() async via GitHub API
├── repertorios.rs       ← CRUD repertorios.json + cargar desde Excel
├── lector.rs            ← leer Excel AUTOFIN SP (calamine)
├── transformador.rs     ← lógica pura AUTOFIN (sin deps externas)
├── escritor.rs          ← escribir planilla masivos (rust_xlsxwriter)
└── planilla_cmd.rs      ← comandos Tauri para cargar/generar planilla
```

---

## Crates Rust usados

| Crate | Propósito |
|---|---|
| `serde` / `serde_json` | Serialización JSON |
| `calamine` | Leer Excel (.xlsx) |
| `rust_xlsxwriter` | Escribir Excel con formato |
| `reqwest` | HTTP (licencia + futuro SIGN+) |
| `regex` | Expresiones regulares |
| `chrono` | Fechas (serial Excel → YYYY-MM-DD) |
| `base64` | Decodificar respuesta GitHub API |
| `dirs` | Rutas de AppData por plataforma |
| `anyhow` | Manejo de errores interno |
| `tokio` | Runtime async (requerido por Tauri) |
| `notify` | (pendiente) Vigilador carpeta escáner |

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

## Módulo Firma Electrónica (pendiente migrar)

- **Carpeta escáner**: `\\Desktop-14rq1mp\escaner riho 550` (configurable en runtime)
- **Modo manual**: usuario selecciona PDF → ve páginas → llena formulario → envía a SIGN+
- **Modo automático**: vigilador detecta PDF nuevo → OCR → busca en BD → envía solo
- **OCR**: pytesseract lang=eng, 250 DPI — aún en Python
- **SIGN+**: `http://192.168.1.177` — solo red interna notaría
  - Usuario: `JESPINA` / Password: `Cpina2026` (se moverá a keychain en versión Rust)
- **BD de casos** y **BD de repertorios**: ya migradas a Rust — el sidecar sigue leyendo los mismos JSON

---

## Kill switch

Editar `licencia.json` en GitHub: `activo: false` + mensaje. Efectivo inmediato via GitHub API (sin cache CDN). Para restaurar: `activo: true`. La verificación ahora se hace directamente desde Rust (sin pasar por el sidecar Python).

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
| **v2.0.0** | **Migración Fase 1: planilla + casos + repertorios + licencia → Rust puro** |

---

## Para publicar una nueva versión

```bash
git tag v2.0.0
git push origin v2.0.0
```

GitHub Actions compila en ~10 min. **Ya no hay paso PyInstaller** para los módulos migrados. El sidecar Python aún se compila con PyInstaller para el módulo Firma.

Variable de entorno requerida en GitHub Secrets: `TAURI_SIGNING_PRIVATE_KEY`
Llave privada local: `.tauri-key` — backup en OneDrive, NO regenerar.

---

## Configuración SIGN+ (sidecar — pendiente migrar)

```
URL_BASE  = http://192.168.1.177
URL_API   = http://192.168.1.177/app/escrituras_publicas/api/
USUARIO   = JESPINA
PASSWORD  = Cpina2026
```

⚠️ Credenciales en Python — se moverán a Windows Credential Manager cuando se migre a Rust.

---

## Próximos pasos (Fase 2 — módulo Firma en Rust)

1. `pdfium-render` — renderizar páginas PDF para visor y OCR
2. Tesseract como subprocess — bundlear tesseract.exe como recurso Tauri
3. `signplus.rs` — HTTP a SIGN+ con reqwest + multipart
4. `vigilador.rs` — watcher con crate `notify`
5. `firma.rs` — orquestador
6. Eliminar `server.py`, `juan_vivi/`, `server.spec`, `build.py`
7. Actualizar GitHub Actions — solo `cargo tauri build`
