# JUAN-VIVI — Estado del Proyecto
> Actualizado: 2026-07-28 | Protocolo aplicado: v6.3

---

## Qué es este proyecto

**JUAN-VIVI** es una app de escritorio para la **33ª Notaría** que hace dos cosas:
1. **Módulo Repertorios**: procesa planillas Excel AUTOFIN SP y genera planilla masivos para el SRCeI
2. **Módulo Firma Electrónica**: gestiona el envío de PDFs escaneados a SIGN+ (manual y automático)

Arquitectura: **Tauri v2** (ventana + auto-updater) + **Python FastAPI sidecar** (lógica de negocio).
Clasificación protocolo v6.3: **Patrón A** (100% local, sin estado en nube).

---

## Estado actual — TODO COMPLETO Y FUNCIONANDO EN PRODUCCIÓN

| Ítem | Estado |
|---|---|
| Módulo Repertorios (lector, transformador, escritor) | ✅ Funcional — probado con archivos reales |
| 31 tests unitarios del transformador | ✅ Todos pasando |
| Módulo Firma Electrónica (OCR, SIGN+, vigilador) | ✅ Funcional |
| Visor PDF (páginas apiladas, scroll interno) | ✅ |
| Base de datos de casos JSON (`data/casos.json`) | ✅ Persistente entre sesiones |
| Auto-relleno del formulario por número de repertorio | ✅ |
| Modal agregar escritura manualmente | ✅ |
| Dashboard Tauri (frontend/index.html) | ✅ Glassmorphism, dos vistas |
| Kill switch via `licencia.json` en GitHub | ✅ Probado en producción |
| Auto-updater Tauri (tauri-plugin-updater) | ✅ Funcional |
| GitHub Actions — release automático por tag | ✅ Funcional (~10 min por build) |
| Sin procesos zombie al cerrar | ✅ child.kill() en evento destroy |
| Instalador NSIS (.exe) autónomo | ✅ ~120 MB, sin dependencias externas |
| Empaquetado Tesseract dentro del sidecar | ✅ |

---

## Arquitectura técnica

```
JUAN-VIVI/
├── frontend/
│   └── index.html          ← UI completa (Tauri, apiFetch, dialogs nativos)
├── server.py               ← FastAPI sidecar — todos los endpoints
├── server.spec             ← PyInstaller spec del sidecar
├── src-tauri/
│   ├── src/lib.rs          ← Tauri: puerto dinámico, spawn sidecar, kill al cerrar
│   ├── tauri.conf.json     ← NSIS, updater, sidecar declarado
│   ├── capabilities/       ← Permisos Tauri v2
│   └── icons/              ← Ícono JV azul (generado automáticamente)
├── juan_vivi/
│   ├── dominio/
│   │   ├── modelos.py      ← FilaEntrada, FilaSalida, ResultadoProcesamiento
│   │   ├── transformador.py← Lógica pura — 3 casos (normal, COMPRA PARA, COMUNIDAD)
│   │   └── casos_prueba.py ← Hardcodeado original (migrado a base_casos.py)
│   ├── infraestructura/
│   │   ├── lector.py       ← Lee Excel AUTOFIN SP, multi-archivo
│   │   ├── escritor.py     ← Genera planilla masivos con formato
│   │   ├── ocr_lector.py   ← PyMuPDF + pytesseract (lang=eng, 250 DPI)
│   │   ├── signplus.py     ← POST a SIGN+ (login JWT + multipart PDF)
│   │   ├── vigilador.py    ← Polling carpeta escáner (8s intervalo, 12s espera)
│   │   ├── firma_controller.py ← Orquestador del módulo firma
│   │   └── base_casos.py   ← BD JSON persistente de casos de firma
│   └── tests/
│       └── test_transformador.py ← 31 tests unitarios
├── build.py                ← Build en 2 pasos: PyInstaller → cargo tauri build
├── licencia.json           ← Kill switch: {activo, mensaje} — editar en GitHub
├── .tauri-key              ← Llave privada Tauri (NO va al repo, backup en OneDrive)
├── dev.py                  ← Launcher pywebview legacy (desarrollo local sin Tauri)
├── main.py                 ← CLI legacy (procesa Excel sin UI)
└── requirements.txt
```

---

## Versiones publicadas

| Versión | Cambios |
|---|---|
| v1.0.0 | Primera versión — Tauri + sidecar FastAPI |
| v1.0.1 | Kill switch via licencia.json + fix initApi espera sidecar |
| v1.0.2 | Fix polling sidecar antes de checkLicencia |
| v1.0.3 | Fix CDN cache — GitHub API en vez de raw.githubusercontent |
| v1.0.4 | Fix procesos zombie — child.kill() al cerrar ventana |

**Versión actual en producción: v1.0.4**

---

## Reglas de negocio del módulo Repertorios

| Col K | Comportamiento |
|---|---|
| vacío (normal) | Compareciente 2 = NOMBRE / RUT (limpia \xa0) |
| `COMPRA PARA` | Compareciente 2 = NOMBRE PARA / RUT COMPRA PARA |
| `COMUNIDAD` | Compareciente 2 = primer nombre + " Y OTRO" / primer RUT (antes del /) |
| Fila vacía (sin nombre ni RUT) | Aviso y omisión silenciosa |

Compareciente 1 SIEMPRE: `76139506-8` / `AUTOFIN S.A.`

Nombre planilla salida: `Repertorios Masivo SP {numero}.xlsx`
El número se extrae del nombre del archivo Excel ignorando la fecha.

---

## Módulo Firma Electrónica

- **Carpeta escáner**: `\\Desktop-14rq1mp\escaner riho 550` (configurable en runtime)
- **Modo manual**: usuario selecciona PDF → ve páginas → llena formulario → envía a SIGN+
- **Modo automático**: vigilador detecta PDF nuevo → OCR → busca en BD → envía solo
- **OCR**: solo se usa en modo automático (no en manual)
- **SIGN+**: `http://192.168.1.177` — accesible solo desde red interna notaría
  - Usuario: `JESPINA` / Password: `Cpina2026` (hardcodeado en signplus.py)
- **BD de casos**: `data/casos.json` — se crea al primer uso con 3 casos de prueba

---

## Kill switch

Editar `licencia.json` en GitHub directamente:

```json
{ "activo": false, "mensaje": "Mensaje para el usuario." }
```

Efectivo en la próxima apertura del programa. Usa GitHub API (sin cache CDN).
Para restaurar: `activo: true`.

---

## Para publicar una nueva versión

```bash
# 1. Hacer cambios y commitear normalmente
git add . && git commit -m "feat: ..."
git push

# 2. Publicar versión (dispara GitHub Actions → build ~10 min → release automático)
git tag v1.0.5
git push origin v1.0.5
```

Variable de entorno requerida en GitHub Secrets: `TAURI_SIGNING_PRIVATE_KEY`
Llave privada local: `C:\Users\javie\OneDrive\Desktop\OuterHeaven\JUAN-VIVI\.tauri-key`
**Backup de la llave en OneDrive — NO regenerar sin necesidad.**

---

## Configuración SIGN+ (signplus.py)

```
URL_BASE  = http://192.168.1.177
URL_API   = http://192.168.1.177/app/escrituras_publicas/api/
USUARIO   = JESPINA
PASSWORD  = Cpina2026
```

⚠️ Credenciales hardcodeadas — pendiente mover a keychain (Regla 40 protocolo).

---

## Pendiente futuro

| Tarea | Prioridad |
|---|---|
| Mover credenciales SIGN+ a Windows Credential Manager (Regla 40) | Media |
| BD real para casos de firma (reemplazar base_casos.py JSON) | Media |
| Fase 1 BD: cargar SP300 → nombre/RUT/nómina | Baja |
| Fase 2 BD: cruzar con repertorio | Baja |
| Fase 3 BD: renombrar PDF + ordenar por nómina | Baja |
| Instalar spa.traineddata para mejorar OCR en español | Baja |
| Ícono definitivo (reemplazar JV placeholder) | Baja |

---

## Reglas críticas recordatorio

1. Todo en español (variables, funciones, comentarios)
2. Máximo 600 líneas por archivo, 40 líneas por función
3. Los `.xlsx` y `.pdf` nunca van al repo
4. No inventar lógica de negocio
5. Build en 2 pasos obligatorio (Regla 66): PyInstaller ANTES de cargo tauri build
