# Roadmap de Implementación — MVP Xschem

Fases ordenadas por dependencia real. Cada fase produce algo funcional y testeable antes de pasar a la siguiente.

---

## Fase 0: Esqueleto del Proyecto

Antes de escribir lógica, definir la estructura que no cambiará.

- [ ] Crear `pyproject.toml` con entry point `riku = "riku.cli:main"`.
- [ ] Definir el protocolo `RikuDriver` (ABC con `available()`, `diff()`, `normalize()`).
- [ ] Crear `miku doctor` — verifica qué herramientas están instaladas y sus versiones.
- [ ] Configurar `miku.toml` mínimo: `[project] name`, `pdk`, `layout_source`.

**Criterio de salida:** `pip install -e .` funciona, `riku doctor` corre y reporta herramientas faltantes.

---

## Fase 1: GitService — Extracción de Blobs

Motor de extracción de datos sin tocar el working tree.

- [ ] Implementar `GitService` con `pygit2` (no subprocess — ya tiene C backend).
- [ ] `get_blob(commit, path)` → bytes: extrae el contenido de un archivo en cualquier commit.
- [ ] `get_commits(path=None)` → lista de commits: historial, opcionalmente filtrado por archivo.
- [ ] `get_changed_files(commit_a, commit_b)` → lista de paths con tipo de cambio (A/M/D).
- [ ] Para blobs > 50 MB: escribir a `.riku/tmp/<commit_short>_<filename>` en vez de cargar en memoria.
- [ ] Test de estrés: extraer un GDS de > 200 MB sin que RAM suba más de 50 MB sobre el baseline.

**Criterio de salida:** dado dos commits, `GitService` entrega los bytes de cualquier archivo sin hacer checkout.

---

## Fase 2: Parser y Diff Semántico de `.sch`

El aporte diferencial de Riku — esto no existe en ninguna herramienta.

- [ ] Detector de formato: leer primera línea, identificar `xschem version=`, Qucs-S o KiCad. Fallback a diff de texto si no es Xschem.
- [ ] Parser de `.sch` → modelo de objetos:
  - `Component(name, symbol, params: dict, x, y, rotation, mirror)`
  - `Wire(x1, y1, x2, y2, label)`
  - `Net(label)` — derivado de wires con `lab=`
- [ ] `SemanticDiff(sch_a_bytes, sch_b_bytes)` → `DiffReport`:
  - Componentes añadidos / eliminados / modificados (por `name`, ignorando coordenadas)
  - Nets añadidas / eliminadas
  - "Move All" detectado: si > 80% de componentes cambiaron solo coordenadas, reportarlo como reorganización cosmética
- [ ] Serializar `DiffReport` a JSON estructurado.
- [ ] Tests con casos reales: cambio de valor, agregar componente, mover todo, eliminar net.

**Criterio de salida:** dado `anterior.sch` y `actual.sch`, produce JSON que dice exactamente qué cambió en el circuito, sin ruido de coordenadas.

---

## Fase 3: Render Visual (SVG Headless)

Integración opcional con Xschem — solo si está instalado.

- [ ] `XschemAdapter.available()` — detecta `xschem --version`, verifica ≥ 3.1.0.
- [ ] `XschemAdapter.render_svg(sch_path)` → `svg_path`: invoca `xschem -q --no_x --svg --plotfile out.svg`.
- [ ] `XschemAdapter.diff_visual(sch_a, sch_b)` → tuple de SVGs: render de ambas revisiones para side-by-side.
- [ ] Caché: clave `SHA256(blob_hash + xschem_version)` → SVG guardado en `~/.cache/riku/ops/<key>/render.svg`.
- [ ] Si Xschem no está: el adapter reporta `available() = False` y el core usa diff semántico en texto como fallback. No error, degradación elegante.

**Criterio de salida:** `riku diff HEAD~1 HEAD amplifier.sch` produce dos SVGs side-by-side + JSON de diff semántico.

---

## Fase 4: CLI Funcional

Conectar todo en un comando usable desde terminal.

- [ ] `riku diff [commit_a] [commit_b] <archivo>` — orquesta Fases 1 + 2 + 3.
- [ ] `--format=text` → diff semántico en texto legible.
- [ ] `--format=json` → `DiffReport` JSON (para scripts y CI).
- [ ] `--format=visual` → abre SVGs side-by-side (si Xschem disponible).
- [ ] `riku log <archivo>` → historial de commits que tocaron ese archivo con resumen de diff semántico.
- [ ] Manejo de errores: archivo no encontrado en ese commit, formato no soportado, Xschem no instalado.

**Criterio de salida:** flujo completo end-to-end funciona desde terminal. Un ingeniero puede usarlo en su repo de chips real.

---

## Fase 5: GUI Mínima (Qt)

Solo después de que el CLI funcione y haya feedback real.

- [ ] Ventana principal: panel izquierdo con historial de commits (GitService).
- [ ] Panel central: visor SVG ligero usando `QGraphicsView` + `QGraphicsSvgItem`.
  - *Nota técnica:* Evitar `QWebEngineView` en el MVP para no heredar la dependencia de Chromium (~150MB) y mantener compatibilidad con entornos mínimos.
  - *Contingencia:* Solo migrar a Chromium si los SVGs de Xschem presentan artefactos críticos de renderizado (fuentes/filtros) que Qt Svg no maneje.
- [ ] Panel derecho: `DiffReport` formateado (componentes cambiados, nets, tipo de cambio).
- [ ] Todo cálculo en `QThread` — el main thread (UI) nunca bloquea.
- [ ] Selección A/B de commits para comparar directamente desde el árbol de Git.

**Criterio de salida:** la GUI reproduce lo que hace el CLI, con UX visual.

---

## Lo que queda fuera del MVP (poscondiciones)

Estas cosas están investigadas pero no entran hasta validar el flujo base:

- **Merge automático de `.sch`:** requiere resolver conflictos de red, no trivial.
- **Diff semántico con `spice_sym_def` resuelto:** requiere parsear símbolos externos.
- **Caché L2 (S3/MinIO):** solo si el equipo crece o el CI empieza a escalar.
- **Rust para streaming GDS:** cuando se mida que `klayout.db` es el cuello de botella real.
- **Git diff driver publicado:** cuando el CLI esté estable, registrarlo como `[diff "xschem"]` en `.gitattributes`.
