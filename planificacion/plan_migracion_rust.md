# Migración a Rust — Estado y Hoja de Ruta

## Estado por fase (2026-04-20)

| Fase | Descripción | Estado |
|------|-------------|--------|
| 1 | Base del proyecto — crate, dependencias, estructura | ✅ completo |
| 2 | Tipos y contratos — modelos, traits, ports | ✅ completo |
| 3 | Capa Git — blobs, commits, historial con git2 | ✅ completo |
| 4 | Parser de Xschem — componentes, wires, nets | ✅ completo |
| 5 | Diff semántico — added/removed/modified/Move All | ✅ completo |
| 6 | CLI — diff, log, doctor, salidas text/json/visual | ✅ completo |
| 7 | Render y anotación SVG — caché, coordenadas, mooz | ✅ completo |
| 8 | Paridad con Python — tests comparativos | ✅ completo |
| 9 | Integración con gdstk/rust | ⏳ pendiente |

## Lo que está implementado

- **CLI**: `riku diff`, `riku log`, `riku doctor` con salidas `text`, `json` y `visual`
- **Parser**: `.sch` de Xschem — componentes, wires, nets, propiedades
- **Diff semántico**: detecta Added/Removed/Modified, redes, Move All cosmético
- **Git**: lectura de blobs y commits con `git2` sin modificar el working tree
- **Render**: invoca `xschem` como proceso externo, caché por SHA256
- **SVG annotator**: transforma coordenadas esquemático→SVG con mooz calibrado desde endpoints de wires
- **Parity tests**: `tests/parity.rs` ejecuta Python y Rust y compara JSON

## Dependencias activas

`clap`, `dirs`, `git2`, `regex`, `serde`, `serde_json`, `sha2`, `tempfile`, `thiserror`, `which`

## Pendiente

- **Fase 9**: integración con `gdstk/rust`
  - Decidir si workspace compartido, dependencia local o API
  - No acoplar hasta cerrar cualquier deuda de paridad

- **Deuda técnica menor**:
  - Tests de integración reales con `#[test]` y assets en vez de scripts manuales
  - Stress test con GDS >200 MB (criterio original de fase 1, nunca validado)
  - Retiro formal del núcleo Python (cuando paridad sea total y equipo lo decida)

## Arquitectura de referencia

```
riku_rust/
  src/
    main.rs
    cli.rs                  ← clap, subcomandos
    core/
      models.rs             ← tipos de dominio
      ports.rs              ← traits GitRepository, etc.
      driver.rs             ← trait RikuDriver, DriverDiffReport
      registry.rs           ← get_drivers(), get_driver_for()
      git_service.rs        ← impl git2
      analyzer.rs           ← analyze_diff()
      semantic_diff.rs      ← diff semántico
      svg_annotator.rs      ← annotate(), Transform, mooz
    adapters/
      xschem_driver.rs      ← impl RikuDriver para xschem
    parsers/
      xschem.rs             ← parse(), detect_format()
  tests/
    parity.rs               ← comparación Python vs Rust
```

## Notas de coordinación

- `xschem` se invoca con `--tcl` + `--command` (TCL inline, no archivo)
- `RIKU_ORIGINS_PATH` se pasa vía env al proceso xschem; TCL lo lee con `$env(...)`
- En Docker, xschem está en `/foss/tools/bin/`; el PATH minimal de `docker exec` no lo incluye — es esperado
- `mooz` se calibra desde endpoints de wires SVG (paths `MxLy`), no desde posiciones tipográficas de texto
