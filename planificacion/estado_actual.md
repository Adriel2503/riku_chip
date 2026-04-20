# Estado Actual del Proyecto — Riku

Actualizado: 2026-04-20

---

## Qué funciona hoy

### Diff semántico de Xschem (`.sch`)

- Parser de archivos `.sch` con soporte de atributos multilinea.
- Diff semántico: detecta componentes added/removed/modified, nets added/removed.
- Detección de "Move All" (reorganización cosmética — no cambia el circuito).
- CLI: `riku diff <a> <b> archivo.sch` en los tres formatos: text, json, visual.

### Diff visual

- Render SVG headless vía Xschem TCL (`zoom_full` + `print svg`).
- Cache por SHA256(version + contenido) — miss ~800ms, hit ~0.2ms (speedup ~3500x).
- Extracción de `xorigin`/`yorigin` en el momento del render vía `xschem get xorigin`.
- Calibración de coordenadas: `mooz` calculado desde wire endpoints del SVG con error `<0.01px`.
- Bounding boxes de color sobre componentes cambiados, ancladas al texto del nombre en el SVG (error 0px por definición).
- Trayectos de wires para nets añadidas (verde) y eliminadas (rojo).

### Historial semántico

- `riku log archivo.sch --semantic` — resumen `+N -N ~N` por commit.
- `riku doctor` — verifica disponibilidad de herramientas EDA.

### Tests y benchmarks

| Script | Qué mide | Resultado |
|---|---|---|
| `bench_parser.py` | Parser .sch | 7µs/comp, 6.9ms para 1000 comps |
| `bench_semantic_diff.py` | Diff semántico | <1ms para <100 comps |
| `bench_git_service.py` | pygit2 throughput | 1.39ms/blob, 2.27ms sweep 20 commits |
| `bench_svg_cache.py` | Cache hit/miss | hit 0.2ms vs miss 800ms |
| `bench_log_semantic.py` | riku log --semantic | 38ms/diff, 717ms para 20 commits |
| `bench_svg_annotator.py` | _fit_transform + annotate | <5ms para 100 comps |

---

## Qué no está implementado todavía

### Drivers pendientes

- **KLayout** (`.gds`, `.oas`) — diff geométrico XOR, render de layout.
- **Magic** (`.mag`) — diff de celdas, render.
- **NGSpice** (`.raw`) — diff de waveforms, `.meas` semántico.

### Infraestructura

- `riku merge` — merge de archivos EDA con resolución de conflictos semántica.
- `riku blame` — anotación por componente de qué commit lo introdujo.
- `riku ci` — modo batch para CI/CD con output parseable.
- Conversión de test scripts manuales a pytest con assertions — pendiente.
- GDS >200MB stress test (RAM <50MB sobre baseline) — pendiente.
- `miku.toml` — configuración de proyecto, tool paths, etapas CI.

### Diff visual

- Wires de nets modificadas (actualmente solo added/removed).
- Identificar el símbolo completo para hacer el bounding box más preciso.

---

## Criterios de salida de fase actual

La fase de Xschem se considera completa cuando:

1. El parser no falla en ningún `.sch` real del repositorio `caravel_user_project_analog`. (LISTO)
2. El diff semántico detecta todos los cambios relevantes sin falsos positivos de Move All. (LISTO)
3. El diff visual muestra bounding boxes y wires alineados con el SVG real. (LISTO — error <1px)
4. Los tests se convierten a pytest con assertions automáticas. (PENDIENTE)

---

## Próximos pasos sugeridos

1. **Convertir test scripts a pytest** — antes de cualquier migración de stack.
2. **Driver KLayout** — el caso de uso más solicitado. Empezar con diff textual de celdas, agregar XOR geométrico después.
3. **GDS stress test** — validar que el manejo de blobs >50MB funciona en producción antes de publicar.
