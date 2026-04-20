# Principios de Arquitectura — Riku

Reglas de diseño para el desarrollo del MVP, priorizando funcionamiento real sobre abstracción prematura.

---

## 1. EDA-Agnostic Core

El motor de Riku (VCS Engine) es independiente de cualquier herramienta EDA.

- **Regla:** Riku funciona con historial, diff de texto y metadatos aunque no haya herramientas EDA instaladas.
- **Implementación:** Sistema de drivers con protocolo `RikuDriver` definido desde el inicio. Cada driver declara `available()` — si no está, Riku degrada a diff de texto sin bloquearse.
- **Violación a evitar:** Importar `klayout` o invocar `xschem` desde `core/`. Solo los adaptadores tocan herramientas externas.

---

## 2. Cero Checkout para Comparar Versiones

`git checkout` para extraer archivos históricos está **prohibido** — ensucia el working tree y es lento para archivos grandes.

- **Mandatorio:** Extraer blobs directamente con `pygit2` (vía libgit2). Más limpio que `subprocess.Popen` + `git cat-file`, y con C backend.
- **Para archivos < 50 MB** (`.sch`, `.mag`, `.spice`): leer en memoria completo, es correcto.
- **Para archivos ≥ 50 MB** (GDS, OASIS): escribir a temporal en `.riku/tmp/<commit_short>_<filename>` y limpiar después. Un GDS de 2 GB con checkout tarda 10–30 s de I/O y modifica el índice de git — con blob directo se evita todo eso.

---

## 3. El `.sch` No Requiere Streaming

Los archivos `.sch` de Xschem son texto plano, típicamente < 1 MB incluso en esquemáticos grandes. Leer en memoria completo es siempre correcto.

- **Detectar formato antes de parsear:** primera línea `v {xschem version=...}`. Si no coincide, fallback a diff de texto — puede ser Qucs-S (`<Qucs Schematic...>`) o KiCad legacy (`EESchema Schematic File Version N`).
- **Referencias externas (`spice_sym_def`):** el diff semántico correcto requiere resolverlas. En MVP es aceptable no resolverlas; documentarlo como limitación conocida.

---

## 4. Identidad Semántica sobre Coordenadas

El diff de `.sch` compara **qué** hay en el circuito, no **dónde** está en el canvas.

- **Identificador único de componente:** atributo `name` (ej. `name=R1`). Nunca las coordenadas.
- **"Move All" no es un cambio:** si todos los componentes desplazaron coordenadas pero sus valores y conectividad no cambiaron, el diff semántico reporta cero cambios. Esto es exactamente el gap que no cubre ninguna herramienta existente.
- **Cambio real:** `value=10u → value=100u` en `C4`, o un net `lab=VDD` que desaparece.
- **Implementación:** construir un dict `{name → {params, nets}}` de cada revisión y diffear los dicts. Coordenadas excluidas explícitamente.

---

## 5. Caché por Hash de Contenido

Toda operación costosa es una función pura: `resultado = f(blob, parámetros, versión_herramienta)`.

- **Clave:** `SHA256(blob_hash + params_hash + tool_version_string)`.
- **Blob hash:** usar el SHA1 del objeto git directamente — ya lo calcula git.
- **Almacenamiento:** `~/.cache/riku/ops/<key>/` con `result.json` + artefactos (SVG, PNG, JSON de diff).
- **Index:** SQLite para lookups O(1) sin recorrer el filesystem.
- **SVG de `.sch`:** el render headless de Xschem tarda < 1 s, pero se cachea igual para evitar fork de proceso repetido.
- **L2 remota (S3/MinIO):** solo para operaciones > 30 s (GDS XOR, DRC). No implementar en MVP.

---

## 6. UI No Bloquea Nunca

- **Main thread:** solo pintar y responder eventos Qt.
- **Todo cálculo** (git, diff semántico, render SVG, XOR) corre en `QThread` o `multiprocessing.Process`.
- **GIL de Python:** el diff semántico corre en Python puro — usar `ProcessPoolExecutor`, no `ThreadPoolExecutor`, para paralelismo real.
- **Comunicación:** señales Qt (`pyqtSignal`) del worker al main thread. Nunca compartir objetos mutables entre procesos.

---

## 7. Rust: Cuándo Entra

Rust no es necesario para el MVP de esquemáticos — el bottleneck de `.sch` es el fork de proceso de Xschem (< 1 s), no el parser Python.

Rust entra cuando se mida alguna condición concreta:
- Diff GDS > 200 MB tarda > 30 s con `klayout.db` en Python.
- Parser de `.raw` de NGSpice multi-GB necesita baja latencia.
- El streaming de blobs git se convierte en cuello de botella (muy improbable — `pygit2` ya usa C).

Interfaz de entrada: PyO3. El core Python no cambia — solo el módulo de extensión se reemplaza por Rust.
