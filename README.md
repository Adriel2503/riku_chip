# Riku — VCS semántico para diseño de chips

Riku es una herramienta de control de versiones semántico sobre Git para archivos EDA. En lugar de diffs de texto crudo, interpreta los cambios al nivel de componentes, conexiones y nets.

**Implementación: Rust puro. Sin dependencia del binario `xschem`.**

## Características

- `riku diff` — diff semántico entre commits: componentes añadidos/removidos/modificados, nets, cambios cosméticos (Move All)
- `riku log` — historial de cambios semánticos por archivo
- `riku render` — renderiza un esquemático a SVG nativo (sin xschem instalado)
- `riku doctor` — verifica el entorno y drivers disponibles
- Caché de renders por hash SHA-256 del contenido

## Formatos soportados

| Formato | Extensión | Diff | Render |
|---------|-----------|------|--------|
| Xschem  | `.sch`    | ✓    | ✓      |
| KLayout, Magic, NGSpice | `.gds`, `.mag`, `.raw` | — | — |

## Instalación

```bash
git clone https://github.com/riku-chip/riku_chip
cd riku_chip/riku
cargo build --release
# binario en: target/release/riku
```

Requiere Rust 1.75+. No requiere `xschem` instalado.

## Uso

```bash
# Diff semántico entre dos commits
riku diff <commit_a> <commit_b> ruta/archivo.sch

# Salida JSON
riku diff <commit_a> <commit_b> archivo.sch --format json

# Diff visual — abre SVG anotado con los cambios
riku diff <commit_a> <commit_b> archivo.sch --format visual

# Renderizar un archivo a SVG
riku render archivo.sch

# Historial semántico
riku log archivo.sch

# Verificar drivers disponibles
riku doctor
```

## Arquitectura

```
riku/
  src/
    cli.rs              — comandos: diff, log, render, doctor
    core/
      models.rs         — Component, Wire, Schematic, DiffReport
      driver.rs         — trait RikuDriver
      git_service.rs    — acceso a objetos Git via git2
      semantic_diff.rs  — diff semántico de schematics
      svg_annotator.rs  — anotación de SVGs con bounding boxes
    parsers/
      xschem.rs         — delega en xschem_viewer::parser (PEG)
    adapters/
      xschem_driver.rs  — implementa RikuDriver para .sch
  tests/
    basic.rs            — tests de integración
    stress.rs           — benchmarks

examples/
  SH/op_sim.sch         — esquemático de referencia (sky130A)
```

## Dependencias clave

| Crate | Rol |
|-------|-----|
| `xschem-viewer` | Parser PEG + renderer SVG nativo para `.sch` |
| `git2` | Acceso a blobs y commits Git |
| `clap` | CLI |
| `sha2` | Cache key por hash de contenido |

`xschem-viewer` vive en [github.com/carloscl03/xschem-viewer-rust](https://github.com/carloscl03/xschem-viewer-rust) y se importa como dependencia git.

## Detección de PDK

`riku render` detecta automáticamente los sym_paths del PDK leyendo `.xschemrc` en el directorio del proyecto o en `~`. Parsea `PDK_ROOT`, `PDK` y `XSCHEM_LIBRARY_PATH` con el mismo esquema que usa xschem.
