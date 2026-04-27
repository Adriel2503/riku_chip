# Integración GDS en riku_chip — estado y roadmap

Documento de seguimiento del trabajo de integrar el soporte de archivos GDSII (layouts físicos de chips) en riku, el VCS para circuitos integrados.

**Última actualización:** 2026-04-26
**Autor de los cambios documentados:** Dante (Adriel2503)

---

## 1. Contexto y motivación

`riku_chip` es un VCS especializado para diseño de chips IC. Hoy maneja únicamente schematics `.sch` (xschem) vía un driver dedicado escrito por Carlos Cueva. El objetivo de este trabajo es agregar soporte para `.gds` (layouts) — el formato dominante en la industria semiconductor para representar geometría física de chips.

**Por qué GDS:** los archivos `.gds` son el output canónico de cualquier flujo de diseño de IC. Sin diff visual de GDS, riku queda limitado a comparar schematics — la mitad de la historia. Un commit típico cambia `.sch` y `.gds` juntos; ver solo uno fragmenta la revisión.

**Diferencia clave respecto a xschem:** schematics son texto editable que se puede parsear y diff-ear semánticamente (componentes, nets). GDS es geometría binaria — el diff útil es **geométrico** (qué polígonos cambiaron en qué capa), no estructural.

---

## 2. Arquitectura

Tres capas con contratos estables (ver `docs/arquitectura_gds.md` para el detalle completo):

```
┌─────────────────────────────────────────────────────────────┐
│  riku-gui  (egui+wgpu nativo, NO browser)                   │
│    ├─ XschemBackend (ya enchufado)                          │
│    └─ GdsBackend (pendiente, Bloque D)                      │
└─────────────────────────────────────────────────────────────┘
        │                              │
        ▼                              ▼
┌──────────────────────┐    ┌──────────────────────────────────┐
│  riku (CLI + core)   │    │  viewer-core (trait neutro)      │
│  ├─ XschemDriver     │    │  ├─ ViewerBackend (async)        │
│  └─ GdsDriver        │    │  ├─ Scene, DrawElement           │
│      (pendiente,     │    │  └─ BackendInfo                  │
│       Bloque C)      │    └──────────────────────────────────┘
└──────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────┐
│  gds-renderer (SVG + structuras de escena)                   │
│  ├─ RenderScene { commands, highlights, viewport, catalog }  │
│  ├─ HighlightSet { added, removed, modified }                │
│  ├─ render_scene_with_highlights() → SVG                     │
│  └─ Paletas PDK (SKY130, GF180, IHP, Generic)                │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────┐
│  gdstk-rs  (binding cxx → C++ gdstk)                         │
│  external/gdstk/ submódulo de Adriel2503/gdstk_rust          │
│  ├─ Library::open / find_cell / write_gds / top_level        │
│  ├─ Cell::xor_with → XorMetrics (resumen)                    │
│  ├─ Cell::xor_polygons_split → XorSplit { added, removed }   │
│  ├─ Library::layers → Vec<GdsTag>                            │
│  ├─ OwnedPolygon { layer, datatype, points }                 │
│  └─ gds_info() peek rápido                                   │
└──────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────┐
│  gdstk C++ (vendored, compilado por build.rs)                │
│  Boolean ops, Clipper, GDSII reader/writer                   │
│  Deps: zlib + qhull (vcpkg en Windows, pkg-config en Unix)   │
└──────────────────────────────────────────────────────────────┘
```

**Patrón clave:** la GUI usa **dos rutas paralelas** para visualización:

- **Ruta rica:** xschem-specific con `ResolvedScene` + diff semántico + fantasmas pintados con `sch_painter.rs`.
- **Ruta neutra:** `ViewerBackend` async/cancellable. Permite agregar formatos sin tocar la GUI core. Es la ruta correcta para GDS.

---

## 3. Lo que ya está hecho

### 3.1 `gdstk-rs` — Fase 9 (commit `4a1134d` en `Adriel2503/gdstk_rust`)

Tres adiciones que desbloquean el diff geométrico:

- **`Cell::xor_polygons_split(other, layer) -> XorSplit { added, removed }`** — devuelve los polígonos del diff en lugar de solo métricas agregadas. Internamente hace dos `gdstk::boolean(_, _, Operation::Not)` — uno por dirección. Necesario para pintar verde/rojo en la UI.
- **`Library::layers() -> Vec<GdsTag>`** — descubrimiento de capas con cache lazy. Evita que el caller cruce `polygon_array` N veces solo para listar layers.
- **`OwnedPolygon`** — tipo POD sin lifetime (`Clone+Send+Sync`) para cruzar fronteras de crate. Reemplaza al `Polygon<'a>` lifetime-bound cuando hay que pasar geometría a otros crates.

**Tests:** 35/35 pasan, incluyendo 4 nuevos: self-XOR vacío, roundtrip estable, invariante `|added|+|removed| ≈ xor_with.area`, layers deduplicado y ordenado.

### 3.2 Migración a submódulo

`gdstk/` movido a `external/gdstk/` y registrado vía `.gitmodules`. Mismo patrón que `external/xschem-viewer-rust` (el repo de Carlos). Beneficios:

- Versión de gdstk fijada por SHA en cada commit de riku_chip → reproducible.
- `Adriel2503/gdstk_rust` queda como repo independiente, clonable solo.
- Elimina el "nested unmanaged" que tenía riku_chip antes.

**Workflow de actualización futura:**
1. `cd external/gdstk && git push` → actualiza repo de gdstk.
2. `cd .. && git add external/gdstk && git commit "bump gdstk" && git push` → actualiza el SHA en riku_chip.
3. Carlos hace `git pull && git submodule update --recursive` → recibe los cambios.

Recomendado: `git config submodule.recurse true` para que `git pull` solo alcance.

### 3.3 Bloque A — preparación de workspace

5 archivos modificados:

- **`Cargo.toml` raíz**: activados `gds-renderer` y `external/gdstk/rust` como workspace members.
- **`gds-renderer/Cargo.toml`**: activada dep `gdstk-rs`.
- **`riku-gui/Cargo.toml`**: activadas deps `gds-renderer` y `gdstk-rs`.
- **`gds-renderer/src/scene.rs`**: borrado el `OwnedPolygon` duplicado, ahora `pub use gdstk_rs::OwnedPolygon`. Cero cambios en callers (compat.rs, renderer.rs, viewport.rs siguen importando `crate::scene::OwnedPolygon`).
- **`riku/src/core/domain/models.rs`**: agregadas variantes `Gds` a `FileFormat` y `DriverKind` con sus brazos `Display`.

**Verificación:** `cargo check --workspace -j 1` pasa limpio. Ver §6 para el bug MSVC2019 que requiere `-j 1`.

### 3.4 Lo que ya estaba (trabajo previo del usuario)

Vale enumerar lo que existía antes de esta sesión, para no duplicarlo:

- **`gds-renderer/`** completo: ~950 LOC. Genera SVG con grupos por layer, paletas PDK, highlights, viewport con bbox + scale.
- **`viewer-core/`**: trait `ViewerBackend` + tipos neutros (`Scene`, `DrawElement`, `BoundingBox`).
- **`riku-gui/`**: infraestructura `Vec<Arc<dyn ViewerBackend>>` lista, hoy registra solo `XschemBackend` en `app.rs:124`.

---

## 4. Lo que falta — bloques pendientes

### Bloque B — `Library::from_bytes` en gdstk-rs

**Problema que resuelve:** hoy `Library::open(path)` requiere un archivo en disco. El `RikuDriver::diff(content_a: &[u8], content_b: &[u8], ...)` recibe bytes directos del blob git. Sin `from_bytes`, el `GdsDriver` tiene que escribir tempfiles cada vez, lo cual es feo y lento.

**Diseño:**

```rust
impl Library {
    pub fn from_bytes(data: &[u8]) -> Result<Self, Error>;
}
```

Implementación interna del shim: escribe los bytes a un tempfile (porque gdstk C++ solo expone `read_gds(path)`, no un reader desde stream), llama `read_gds`, borra el tempfile, retorna el `Library` o un error de I/O.

**Es un PR aparte al submódulo** (`Adriel2503/gdstk_rust`). ~30 líneas en `shims.cpp`, ~15 en `lib.rs`, 1 test (`from_bytes_matches_open` — abrir el mismo GDS por path y por bytes deben dar libraries equivalentes).

**Esfuerzo:** 1-1.5 h. **Bloquea:** Bloque C completo, Bloque D parcialmente.

### Bloque C — `GdsDriver` (CLI funciona)

**Output del bloque:** `riku diff archivo.gds` en terminal devuelve diff geométrico por (cell, layer) y SVG renderizado.

**Archivos:**
- **Nuevo:** `riku/src/adapters/gds_driver.rs` (~200-300 líneas).
- **Modificar:** `riku/src/adapters/registry.rs` (1 línea: agregar `Box::new(GdsDriver::new())`).
- **Modificar:** `riku/Cargo.toml` (agregar deps `gdstk-rs`, `gds-renderer`).

**Implementación de cada método del trait `RikuDriver`:**

| Método | Implementación |
|---|---|
| `info()` | `DriverInfo { name: DriverKind::Gds, available: true, version: "0.1.0", extensions: [".gds".into()] }` cacheado en `OnceLock`, mismo patrón que `XschemDriver` |
| `diff(a, b, hint)` | `Library::from_bytes(a)` × 2 → recorrer `union(lib_a.layers(), lib_b.layers())` × `union(lib_a.cells(), lib_b.cells())` → `cell_a.xor_polygons_split(&cell_b, layer)` por par → construir `Vec<DiffEntry>` (uno por (cell, layer) con cambios) → llamar `gds_renderer::render_scene_with_highlights` para `visual_a`/`visual_b` |
| `render(content, hint)` | `Library::from_bytes` → `scene_from_cell` (top-level) → `render_scene` → `Some(svg_string)` |
| `normalize(content, _)` | `content.to_vec()` (no-op; GDS no se normaliza) |
| `can_handle(filename)` | Default del trait: matchea `.gds` |

**Mapping del diff a `DiffEntry`:**

- Cell solo en A → `{ kind: Removed, element: cell_name, ... }`.
- Cell solo en B → `{ kind: Added, element: cell_name, ... }`.
- Cell en ambos, layer con `added.len() > 0` o `removed.len() > 0` → `{ kind: Modified, element: format!("{cell}/L{layer}.{datatype}"), before: bbox+area+count, after: bbox+area+count }`.
- Cell en ambos, layer sin diferencias geométricas pero con orden de polígonos distinto → `{ kind: Modified, ..., cosmetic: true }`.

**Esfuerzo:** 2-3 h tras Bloque B. **Bloquea:** ergonomía de Bloque D (preferimos `from_bytes` en backend también).

### Bloque D — `GdsBackend` + painter (GUI funciona)

**Output del bloque:** abrir un `.gds` en `riku-gui` lo muestra renderizado con egui, con toggle de capas por checkbox y diff overlay (verde/rojo).

**Decisión arquitectónica:** la GUI **no usa el SVG** que genera `gds-renderer`. Pinta directo con `egui::Painter` desde `RenderScene::commands`. Más rápido para zoom/pan, sin dependencia de `resvg`. El SVG queda solo para CLI / fallback browser.

**Archivos:**
- **Nuevo:** `gds-renderer/src/backend.rs` o crate aparte `gds-viewer/` con `pub struct GdsBackend; impl ViewerBackend for GdsBackend`. Métodos:
  - `accepts(content, path)` → check magic bytes GDS (`0x00 0x06 0x00 0x02` = HEADER record) o sufijo `.gds`.
  - `load(content, path, token)` → `Library::from_bytes` → construir `Scene` neutro de viewer-core (mapping desde `RenderScene::commands` → `Vec<DrawElement>`).
  - `info()` → `BackendInfo { name: "gds", ... }`.
- **Nuevo:** `riku-gui/src/gds_painter.rs` (~150-300 líneas) — análogo a `sch_painter.rs:49-88`. Toma una `Scene` + opcionales `HighlightSet` y la pinta con `ui.painter().add(Shape::convex_polygon(...))` por layer.
- **Modificar:** `riku-gui/src/app.rs:124` — agregar `Arc::new(GdsBackend::new())` al vec.

**UI features mínimas:**
- Checkbox por layer (toggle visibility).
- Pan con drag, zoom con wheel.
- Click en polígono → tooltip con (layer, datatype, area, bbox).
- Si hay diff: panel lateral con lista de cambios (cell + layer + área), click → centrar viewport.

**Esfuerzo:** 4-8 h. **Es el bloque más grande.** Tareas internas:
1. Mapping `RenderScene → viewer_core::Scene` (1-2 h).
2. Painter base sin diff (2-3 h).
3. Painter con highlights (1 h).
4. UI controls (toggle, pan, zoom — 1-2 h).

---

## 5. Decisiones tomadas y pendientes

### Tomadas

- **Submódulo en lugar de subdir.** `gdstk` vive en `external/gdstk` con su propio repo y SHA fijado. No es un nested unmanaged.
- **`OwnedPolygon` único.** El de `gdstk-rs` es el canónico; `gds-renderer` lo re-exporta.
- **GUI nativa egui, no browser.** Aunque `arquitectura_gds.md` mencionaba HTML+JS para Fase 1, la GUI ya saltó a Fase 2. El SVG de `gds-renderer` se mantiene para CLI y fallback, pero la GUI pinta direct con `egui::Painter`.
- **Diff geométrico, no semántico.** A diferencia de xschem (componentes + nets), GDS diff trabaja por (cell, layer) con `xor_polygons_split`.
- **Tests --release crashean por bug MSVC2019.** Workaround: `cargo test --test integration` en debug, o setear `jobs = 1` en `~/.cargo/config.toml`.

### Pendientes

- **Tolerancia/snap en XOR.** Hoy XOR reporta cualquier diferencia ≥ 1 `int64` después del `scaling=1000`. Para GDS reales conviene un epsilon (≥ 10 nm típico). Posibles diseños: parámetro extra a `xor_polygons_split`, o una fase de filtrado post-XOR. **Pendiente para Fase 1.5.**
- **XOR jerárquico.** Hoy el XOR es cell-by-cell. Si una `Reference` se mueve sin cambiar contenido, aparece como "todo cambió". Solución: flatten de cells antes del XOR, o detectar reference moves antes y descartarlas. **Pendiente para Fase 2.**
- **Detección de cells renombradas.** Una cell renombrada aparece como `DELETE + ADD` ruidoso. Solución: hash de contenido (bbox + area + polygon count) + fuzzy matching. **Pendiente.**
- **OASIS (.oas).** Formato más compacto que GDSII, usado por foundries modernas (≤ 7 nm). gdstk lo soporta nativo. Agregar `.oas` a `extensions` del driver una vez que `Library::from_bytes` exista. **Pendiente, bajo riesgo.**
- **Cache de XOR.** Para GDS grandes (>100 MB), XOR puede tardar 30-90 s. Necesario un cache por hash de contenido en `~/.cache/riku/ops/`. **Pendiente para Fase 2.**
- **README de riku_chip.** Agregar prereqs vcpkg/pkg-config y workflow de submódulos (`git clone --recursive`, `git submodule update`). **Pendiente, no bloquea código.**

---

## 6. Problemas conocidos del entorno (no del código)

### Bug MSVC 2019 Build Tools

**Síntomas:**
- `LNK1171: no se puede cargar mspdbcore.dll (código 1455)` al linkear proc-macros (tokio-macros, syn, serde_derive, clap_derive).
- `STATUS_STACK_BUFFER_OVERRUN (0xc0000409)` en `link.exe` o `rustc.exe`.
- `rustc-LLVM ERROR: out of memory / Allocation failed`.

**Causa:** versión 14.29.30133 (abril 2021) de MSVC tiene problemas conocidos con builds paralelos pesados de proc-macros. Cada `rustc` puede consumir 1-3 GB para optimización LLVM; con 8-16 jobs simultáneos y el linker cargando `mspdbcore.dll` por cada uno, agota memoria committeada del sistema.

**Workarounds (orden de menor a mayor esfuerzo):**

1. **Build serial:** `cargo build -j 1` o setear permanente:
   ```toml
   # ~/.cargo/config.toml
   [build]
   jobs = 1
   ```
2. **Linker `lld` de LLVM** (más liviano, no carga `mspdbcore.dll`):
   ```toml
   [target.x86_64-pc-windows-msvc]
   linker = "rust-lld"
   ```
3. **Solución real:** actualizar a Visual Studio Build Tools 2022 (MSVC 14.40+). Visual Studio Installer → desinstalar 2019 BT → instalar 2022 BT con workload "Desktop development with C++".

### DLLs de vcpkg en runtime

Tests y ejemplos de gdstk-rs requieren `qhull_r.dll` y `zlib1.dll` junto al ejecutable cuando se usa el triplet `x64-windows`. Workaround manual:

```bash
cp C:/vcpkg/installed/x64-windows/bin/{qhull_r.dll,zlib1.dll} target/debug/deps/
cp C:/vcpkg/installed/x64-windows/bin/{qhull_r.dll,zlib1.dll} target/debug/examples/
```

Alternativa: instalar con triplet `x64-windows-static` para link estático sin DLLs en runtime.

---

## 7. Cómo seguir

**Próximo bloque sugerido: B (`Library::from_bytes`).** Es prerrequisito directo de C y D. Esfuerzo bajo (1.5 h), PR aislado al submódulo, no toca riku_chip.

Después de B, decidir si:
- **Camino A — terminar CLI primero:** B → C → D. Más lineal. CLI funcional antes de tocar GUI.
- **Camino B — GUI más temprano:** B → D (con tempfile interno temporalmente) → C. Resultado visible más rápido pero GUI sin diff hasta que C esté listo.

**Recomendación:** Camino A. La CLI da feedback rápido sin debug de painter, y `riku diff foo.gds bar.gds` en terminal es ya un hito tangible. La GUI requiere más decisiones de UX que conviene tomar con datos del CLI funcionando.

---

## 8. Referencias

- `docs/arquitectura_gds.md` — contratos detallados entre crates.
- `docs/research/architecture/gdstk_rust_decisiones.md` — decisiones de diseño del binding.
- `docs/research/architecture/gdstk_rust_bindings_migracion.md` — plan original de la migración Python → Rust.
- `docs/research/herramientas/gds_klayout_magic_diff.md` — investigación sobre diff de GDS con KLayout/Magic.
- `external/gdstk/rust/README.md` — setup del binding (vcpkg, pkg-config).

**Repos:**
- riku_chip: https://github.com/riku-chip/riku_chip
- gdstk-rs: https://github.com/Adriel2503/gdstk_rust
- xschem-viewer-rust: https://github.com/carloscl03/xschem-viewer-rust
