# Riku Rust

`riku_rust` es la implementacion oficial en Rust de Riku, un VCS semantico para diseno de chips. Compara esquematicos por estructura, no como diff de texto, empezando por Xschem.

## Que hace

- `diff` semantico entre dos commits de un archivo `.sch`
- `log` del historial por archivo
- `doctor` para verificar herramientas externas
- salida `text`, `json` y `visual`
- render SVG anotado para Xschem
- cache de renders por contenido + version de herramienta

## Estado del proyecto

- La base Rust ya esta implementada.
- La paridad con la version Python fue validada con tests.
- Python queda como referencia historica y de comparacion.
- La integracion con `gdstk/rust` queda para una fase posterior.

## Arquitectura

El proyecto sigue un monolito modular con enfoque hexagonal:

- `core`: modelos, diff semantico, Git, contratos y anotacion SVG
- `parsers`: parseo de formatos EDA, empezando por Xschem
- `adapters`: drivers y acceso a herramientas externas
- `cli`: comandos de usuario

## Requisitos

- Rust stable 1.95 o superior
- `xschem` instalado si quieres usar `--format visual`
- un repositorio Git con archivos `.sch`

## Instalacion

```bash
cargo build
cargo test
```

Para revisar estilo y calidad:

```bash
cargo fmt
cargo clippy
```

## Uso rapido

### Diff semantico

```bash
cargo run -- diff <commit_a> <commit_b> <archivo.sch>
```

Con JSON:

```bash
cargo run -- diff <commit_a> <commit_b> <archivo.sch> --format json
```

Con visual:

```bash
cargo run -- diff <commit_a> <commit_b> <archivo.sch> --format visual
```

### Historial

```bash
cargo run -- log <archivo.sch>
```

Con resumen semantico:

```bash
cargo run -- log <archivo.sch> --semantic
```

### Doctor

```bash
cargo run -- doctor
```

## Dependencias externas

- `xschem` se usa solo para render visual
- Git se lee directamente desde los commits, sin checkout
- la cache de render vive en `~/.cache/riku/ops` o equivalente del sistema

## Notas tecnicas

- El parser de Xschem soporta bloques multilinea y archivos reales del repo.
- El diff semantico distingue cambios funcionales de cambios cosméticos.
- Los renders se cachean con `SHA256` usando version de herramienta + contenido.

## Desarrollo

Si quieres contribuir o seguir migrando modulos:

1. revisa `riku_rust/src/`
2. ejecuta `cargo test`
3. compara comportamiento con la suite de paridad
4. mantén la logica de dominio fuera de la CLI y de la infraestructura

## Roadmap

Pendiente para fases posteriores:

- integracion con `gdstk/rust`
- soporte de mas formatos EDA
- refinamiento adicional del render visual
