# Decisiones Técnicas — Riku

Registro de decisiones de implementación, fixes y su justificación.
Se actualiza a medida que avanza el desarrollo.

---

## Git integration — pygit2 vs subprocess

**Decisión:** usar `pygit2` (bindings a libgit2) en lugar de `subprocess git` o `gitpython`.

**Por qué:**
- `subprocess git` tiene ~50-200ms de overhead fijo por llamada (fork de proceso). Inaceptable para archivos GDS grandes donde ya hay latencia de I/O.
- `gitpython` es un wrapper de subprocess — mismo problema.
- `pygit2` accede directamente a la base de datos de objetos Git via libgit2 (~0.5ms por blob).
- Permite manejar blobs >50MB sin cargar todo en RAM de Python (escribir a `.riku/tmp/`).

**Trade-offs aceptados:**
- Requiere wheel precompilado de libgit2 en Windows — ya declarado en `pyproject.toml`.
- La API de pygit2 es menos legible que un comando git, pero el rendimiento lo justifica.

---

## Cache de versión de Xschem — class-level vs instance-level

**Decisión:** `XschemDriver._cached_info` es un atributo de clase, no de instancia.

**Fix aplicado:** commit `1634271`

**Problema:** `render()` llamaba `self.info()` en cada invocación, incluso en cache hits. `info()` ejecutaba `subprocess.run(["xschem", "--version"])` cada vez — 335ms de overhead por llamada.

**Por qué class-level:** xschem es una herramienta del sistema, su versión no cambia entre instancias del driver ni durante la vida del proceso. Un cache de instancia requeriría que el mismo objeto sobreviva entre llamadas, lo cual no está garantizado.

**Resultado:** cache hit bajó de 335ms → 0.24ms (speedup ~1400x).

---

## Clave de cache SHA256 — sin `.hex()`

**Decisión:** `hashlib.sha256(version.encode() + b"::" + content)` en lugar de `hashlib.sha256(f"{version}::{content.hex()}".encode())`.

**Fix aplicado:** commit `1634271`

**Por qué:** `content.hex()` convierte cada byte en 2 caracteres ASCII, duplicando el tamaño del dato antes de hashearlo. Para un archivo de 200KB esto genera 400KB de string intermedio en heap. La concatenación de bytes es directa y no genera copia.

---

## Adapter genérico — sin paths hardcodeados de iic-osic-tools

**Decisión:** `XschemDriver` depende solo de que `xschem` esté en el PATH. No ejecuta `sak-pdk sky130A` ni asume rutas de Docker.

**Por qué:** `sak-pdk` es configuración del entorno del usuario, no responsabilidad de Riku. Hardcodear rutas de iic-osic-tools rompería el adapter en cualquier otra instalación de xschem (Arch Linux, Nix, conda-forge).

**Consecuencia:** el usuario es responsable de configurar su entorno antes de invocar Riku.

---

## Orquestador de diff — función libre en `analyzer.py`

**Decisión:** la capa que conecta `GitService` + `registry` + `DriverDiffReport` será una función libre en `riku/core/analyzer.py`, no un método de `GitService`.

**Por qué:** `GitService` no debe conocer drivers EDA — su responsabilidad es acceso a objetos Git. Mezclar ambas responsabilidades violaría separación de concerns y dificultaría testear cada capa por separado.

**Interfaz planeada:**
```python
def analyze_diff(repo_path, commit_a, commit_b, file_path) -> DriverDiffReport
```

---

## Encoding UTF-8 en scripts de terminal (Windows)

**Fix aplicado:** `sys.stdout.reconfigure(encoding="utf-8", errors="replace")` al inicio de cada script de test/benchmark.

**Por qué:** la consola de Windows usa cp1252 por defecto. Los mensajes de commit con tildes (é, ó, ñ) lanzaban `UnicodeEncodeError` o se mostraban como `?`. El reconfigure fuerza UTF-8 sin romper el output en ningún entorno.

---

## CLI — Typer vs Click

**Decisión:** usar Typer como framework de CLI.

**Por qué:**
- Typer es un wrapper de Click que usa type hints de Python para inferir argumentos — menos boilerplate para los mismos 3 comandos.
- Internamente es Click: si en el futuro se necesita control fino, el acceso está disponible.
- Viene con Rich integrado, útil cuando se implemente output con colores.
- ~16M descargas semanales, mantenido por el autor de FastAPI — riesgo de abandono bajo.

**Trade-off aceptado:** añade Rich como dependencia transitiva. Aceptable porque Rich será útil en la fase de output de CLI.

**Eficiencia:** idéntica a Click — el overhead de parseo de argumentos es microsegundos vs el trabajo real del backend.

---

## Encoding en CLI — doble filtro

**Decisión:** dos capas de protección contra `UnicodeEncodeError` en Windows (cp1252):

1. **Primera capa:** eliminar caracteres no soportados por cp1252 del código fuente — `—` → `-`, `✓`/`✗` → `[ok]`/`[x]`. Elimina el problema de raíz.
2. **Segunda capa:** `sys.stdout.reconfigure(encoding="utf-8", errors="replace")` al inicio de `cli.py`. Red de seguridad para cualquier carácter no-ASCII que se cuele en el futuro.

**Por qué este orden:** primero resolver la causa, luego agregar protección. `errors="replace"` como último recurso sustituye caracteres por `?` en lugar de lanzar excepción — nunca falla, pero puede degradar output.

**En Linux/Docker:** cp1252 no existe, UTF-8 es el default. El `reconfigure` es no-op en esos entornos.

---

## Archivo nuevo o borrado en `analyze_diff`

**Decisión:** si el archivo no existe en `commit_a` o `commit_b`, `_safe_get_blob()` retorna `b""` en lugar de propagar `KeyError`.

**Por qué:** un archivo nuevo es un caso válido, no un error — el driver lo interpreta correctamente como todos los componentes `added`. Lanzar excepción forzaría al caller a manejar un caso que semánticamente no es un error.

**Consecuencia:** `XschemDriver.diff(b"", content_b)` retorna todos los componentes de B como `added`. `diff(content_a, b"")` retorna todos como `removed`.

---

## Diff visual — Opción B (bounding boxes por coordenadas)

**Decisión:** el diff visual superpone bounding boxes de color sobre el SVG de Xschem, centrados en las coordenadas del componente. No se genera un diff pixel a pixel ni se parsea la geometría del símbolo.

**Opciones evaluadas:**
- A — superposición de SVGs (rojo + verde translúcido): confuso con muchos cambios, descartada.
- B — bounding boxes por coordenadas: factible con datos ya disponibles, elegida.
- C — parsear geometría del SVG: frágil ante cambios de Xschem, descartada para MVP.

**Por qué B:** el parser ya guarda `x, y` de cada componente. La transformación `.sch → SVG` es lineal y resoluble con mínimos cuadrados usando los nombres de componentes visibles en ambos lados.

**Limitación aceptada:** el box se centra en el texto del nombre del componente, no encierra el símbolo completo. Es suficiente para identificar visualmente qué cambió.

**Colores:** verde = added, rojo = removed, amarillo = modified.

---

## Transformacion de coordenadas .sch → SVG

**Hallazgo:** la transformación de Xschem es `svg = mooz * sch + offset`, donde `mooz` depende del zoom al momento del export. No es un factor fijo.

**Solución:** calcular `mooz` y los offsets empíricamente por mínimos cuadrados, cruzando los nombres de componentes que aparecen en el SVG (`<text fill="#cccccc">`) con sus coordenadas en el `Schematic` parseado. Requiere al menos 2 puntos.

**Resultado medido:** con `zoom_full` y viewport 900×532, `mooz ≈ 0.674`. Error de predicción: ~7px en X, ~3px en Y — dentro del radio del bounding box.

**Implementado en:** `riku/core/svg_annotator.py`, función `_fit_transform()`.

---

## Fix parser xschem.py — atributos multilinea

**Fix aplicado:** commit `dec26b6`

**Problema:** el regex original `[^}]*` no cruzaba líneas. Componentes con atributos multilinea como:
```
C {pfet.sym} 550 -400 0 1 {name=M1
L=\{l1\}
W=\{w1\}}
```
no eran parseados — el nombre `M1` no se extraía y el componente se ignoraba silenciosamente.

**Fix:** agregar flags `re.MULTILINE | re.DOTALL` al regex de componentes y usar `finditer()` sobre el texto completo en lugar de iterar línea por línea.

**Impacto:** el parser ahora captura todos los componentes de diseños reales sky130, que usan atributos multilínea extensamente.

---

## Formatos de salida CLI — text, json, visual

**Decisión:** `riku diff` soporta tres formatos via `--format`:
- `text` — salida legible por humanos, default.
- `json` — `DriverDiffReport` serializado, para scripts y CI.
- `visual` — genera SVG anotado y lo abre en el visor del sistema.

**Por qué tres formatos:** cada contexto de uso requiere un formato distinto. Un ingeniero en terminal quiere texto; un script de CI quiere JSON parseble; una revisión de diseño quiere el SVG.

**`riku log --semantic`:** agrega un resumen `+N -N ~N` de cambios semánticos por commit, comparando cada commit con el anterior. Solo disponible cuando se filtra por archivo.

---

## Blobs grandes (>50MB) — escritura a `.riku/tmp/`

**Decisión:** si un blob supera 50MB, `GitService.get_blob()` lo escribe a `.riku/tmp/<short_id>_<filename>` y lanza `LargeBlobError` con la ruta.

**Por qué:** cargar un GDS de 200MB en RAM de Python para luego pasárselo a KLayout (que también lo cargará) duplica el uso de memoria innecesariamente. El caller puede decidir si usar la ruta del archivo temporal o ignorar el error.

**Threshold:** 50MB — valor conservador que cubre todos los .sch y la mayoría de .gds pequeños, pero protege contra GDS de chips completos.

---

## Render SVG — sin toggle_colorscheme

**Fix aplicado:** commit `93ac2c0`

**Problema:** el comando TCL incluía `xschem toggle_colorscheme` para forzar fondo blanco. Esto cambiaba el color de los textos de nombres de componentes de `#cccccc` a `#222222`, haciendo que `_extract_name_positions()` no encontrara ningún nombre (0 posiciones).

**Fix:** eliminar `toggle_colorscheme` del comando de render. El SVG queda con el colorscheme por defecto de Xschem, que incluye `#cccccc` para nombres de instancia.

---

## Env var para path TCL — RIKU_ORIGINS_PATH

**Fix aplicado:** commit `93ac2c0`

**Problema:** interpolar la ruta del archivo `origins.txt` directamente en el string TCL con `shell=True` causaba que el shell interpretara `$` como variable de shell (PID en bash), corrompiendo la ruta.

**Fix:** pasar la ruta vía variable de entorno `RIKU_ORIGINS_PATH` y accederla desde TCL como `$env(RIKU_ORIGINS_PATH)`. Esto es la forma estándar de pasar datos al intérprete TCL sin problemas de escape.

```python
env = {**os.environ, "RIKU_ORIGINS_PATH": str(origins_path)}
# TCL: set _f [open $env(RIKU_ORIGINS_PATH) w]
```

---

## Calibracion de mooz desde wire endpoints (no textos)

**Fix aplicado:** commit `93ac2c0`

**Problema:** estimar `mooz` desde los textos `#cccccc` de nombres de componentes introducía un sesgo tipográfico variable por símbolo (~0.5%). Los textos no coinciden con el anchor del símbolo — Xschem los coloca con un offset que depende del símbolo. Este sesgo causaba un desfase visible en los trayectos de wires de ~2-10px.

**Medición del sesgo:** `mooz` desde textos = 0.4541, `mooz` real desde wire endpoints = 0.4517. Diferencia de 0.5% que se amplifica con la distancia.

**Solución:** en `_lstsq_fixed_origins()`, una vez obtenido un `mooz` preliminar desde textos, se matchean los endpoints de wires del `.sch` contra los paths `M x yL x y` del SVG usando búsqueda del vecino más cercano con umbral de 8px. Con los pares validados se recalcula `mooz` desde ambos ejes X e Y. Error final `<0.01px`.

**Move All — detección de reorganización cosmética:** si >80% de los componentes comunes entre dos versiones solo cambian en coordenadas (sin cambios de símbolo, parámetros ni nombre), se marca el diff completo como `is_move_all=True` y se suprime la lista individual de cambios. Un "Move All" en Xschem es cosmético — no cambia el circuito.
