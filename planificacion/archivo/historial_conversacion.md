# Historial de Conversacion del Proyecto Riku

Este documento resume la conversacion de trabajo sobre `riku_chip`, con foco en la migracion a Rust, la documentacion del proyecto y las decisiones tecnicas que se tomaron durante el proceso.

## 1. Contexto general

Al inicio se reviso el estado del repo y se confirmo que el proyecto principal seguia alineado con `main` de GitHub. A partir de ahi se aclaro que existia una carpeta Rust nueva, `riku_rust`, creada para migrar la logica principal de `riku` desde Python.

Tambien se verifico que Git no versiona carpetas vacias, por lo que una carpeta nueva como `riku_rust` solo aparece si contiene archivos reales.

## 2. Analisis del proyecto Python

Se pidio una revision de `C:\Users\ariel\Documents\riku_chip\riku` para entender que hacia y como funcionaba.

### Resultado del analisis

- `riku/cli.py` era el punto de entrada principal con Typer.
- `riku/core/analyzer.py` coordinaba la lectura desde Git y el uso del driver adecuado.
- `riku/core/git_service.py` usaba `pygit2` para leer blobs y commits sin hacer checkout.
- `riku/core/driver.py` definia el contrato de drivers.
- `riku/core/semantic_diff.py` implementaba el diff semantico.
- `riku/core/svg_annotator.py` hacia la anotacion visual de SVG.
- `riku/parsers/xschem.py` parseaba `.sch` de Xschem.
- `riku/adapters/xschem_driver.py` y `riku/adapters/xschem_adapter.py` contenian la logica de render y diff para Xschem.

La conclusion fue que la logica de Python estaba concentrada en pocos modulos y que podia migrarse a Rust sin bloquear el diseño general.

## 3. Decision de migracion a Rust

Se acordó que:

- `riku_rust/` seria el nuevo nucleo en Rust.
- La arquitectura recomendada seria un **monolito modular con enfoque hexagonal**.
- Python quedaria como referencia historica y de paridad.
- La integracion con `gdstk/rust` se dejaria para el final.

Se discutio tambien que la migracion no se haria de un solo golpe, sino por fases.

## 4. Arquitectura Rust acordada

Se definio una estructura con capas:

- `core`
  - modelos
  - diff semantico
  - Git
  - contratos
  - anotacion SVG
- `parsers`
  - parseo de formatos EDA
  - primer objetivo: Xschem
- `adapters`
  - drivers
  - acceso a herramientas externas
- `cli`
  - comandos de usuario

El estilo de codigo se decidio bajo la idea de **clean code**, pero la arquitectura como tal se mantuvo hexagonal/modular.

## 5. Fases del plan de migracion

Se genero y guardo un plan maestro en Markdown, dividido por fases.

### Fase 1

- Crear `riku_rust/`
- dejar el crate funcionando
- configurar `Cargo.toml`
- fijar estructura base
- verificar compilacion y tests

Se considero cerrada cuando el crate compilo y los tests pasaron.

### Fase 2

- Tipos y contratos del nucleo
- modelos mas estrictos
- puertos para Git, parser y renderer
- error comun

### Fase 3

- Capa Git
- lectura de blobs
- commits y cambios entre revisiones
- deteccion de renames
- manejo de blobs grandes

### Fase 4

- Parser de Xschem
- reescritura a parser por lineas/bloques
- soporte de multilinea
- compatibilidad con archivos reales como `examples/SH/op_sim.sch`

### Fase 5

- Diff semantico
- diferenciacion de cambios cosméticos y funcionales
- `Move All`

### Fase 6

- CLI y salida
- `diff`, `log`, `doctor`
- `text`, `json`, `visual`
- helpers de traduccion de reportes

### Fase 7

- Render y anotacion visual
- cache por hash
- `render.json`
- mejora de bounding boxes y estilos de anotacion

### Fase 8

- Paridad completa con Python
- harness de comparacion
- comparacion de CLI, doctor, render y diff
- alineacion de comportamientos importantes

### Fase 9

- Integracion futura con `gdstk/rust`
- pendiente hasta que el nucleo este estable

## 6. Dependencias Rust

Se revisaron y actualizaron las dependencias Rust del proyecto.

### Crates principales

- `clap`
- `dirs`
- `git2`
- `once_cell`
- `regex`
- `serde`
- `serde_json`
- `sha2`
- `tempfile`
- `thiserror`
- `which`

Tambien se discutio la diferencia entre dependencias esenciales y opcionales.

### Decisiones sobre dependencias

- Se mantuvo `dirs` para cache de usuario.
- Se mantuvo `which` para localizar `xschem`.
- Se mantuvo `sha2` para cache por contenido + version.
- Se reviso que el Rust local ya estaba en la linea estable correcta.

Tambien se aclaro la diferencia entre `Cargo.toml` y `Cargo.lock`, analogamente a `pyproject.toml` y `uv.lock` en Python.

## 7. Problemas de toolchain y edition

Se encontro un problema por el cual una version de Cargo no aceptaba `edition = "2026"`. La conclusion fue:

- `edition` no es el año actual, sino una edicion del lenguaje Rust.
- Las ediciones validas eran `2015`, `2018`, `2021` y `2024`.
- Se corrigio `edition = "2026"` a `edition = "2024"`.

Tambien se investigaron los cambios entre `2021` y `2024`:

- cambios en `rustfmt`
- cambios en macros `expr`
- funciones que pasan a ser `unsafe`
- resolver de Cargo consciente de `rust-version`

La conclusion fue que, para este proyecto, `2024` era la mejor opcion.

## 8. Documentacion creada

Se trabajo mucho sobre la documentacion del proyecto Rust.

### README de `riku_rust`

Se reescribio varias veces para que incluyera:

- que hace el proyecto
- arquitectura
- estado actual
- compilacion
- uso rapido
- ejemplos de Windows y Linux
- instalacion de Rust
- binario local
- uso como comando instalado
- dependencias externas

Tambien se dejo claro que:

- `cargo run -- ...` es el flujo de desarrollo
- `target/debug/riku.exe` o `target/debug/riku` es la ejecucion nativa local
- `cargo install --path .` permite instalarlo como comando global

### Documentacion de migracion

Se crearon o guardaron varios documentos en `planificacion/`, incluyendo:

- `plan_migracion_rust.md`
- `plan_migracion_rust_checklist.md`
- `plan_migracion_rust_indice.md`
- fases individuales por archivo

### Documento del CLI futuro

Tambien se creo `planificacion/cli_futuro.md`, con la idea de documentar:

- el CLI actual
- una posible evolucion hacia modo interactivo
- comandos tipo `/diff`, `/log`, `/doctor`
- compatibilidad con scripts

## 9. Ajustes en el README principal

Se mejoro el `README.md` del repo raiz para enlazar la documentacion de migracion a Rust y el futuro del CLI.

Ese README quedo como entrada general del proyecto, mientras que `riku_rust/README.md` quedo como documentacion especifica del crate Rust.

## 10. Git ignore y artefactos

Se reviso el `.gitignore` del repo.

### Cambios importantes

- Se agrego `/riku_rust/target/` para ignorar artefactos de compilacion Rust.
- Se saco `riku_rust/target/` del indice de Git, ya que habia quedado accidentalmente trackeado.

Se detecto tambien que `*.svg` quedaba ignorado de forma global, lo cual se considero potencialmente demasiado amplio para el futuro.

## 11. Autor y metadatos

Se pidio agregar el credito de autor `Ariel Amado Frias Rojas`.

### Donde se agrego

- `Cargo.toml`
  - como metadata oficial del crate
- `src/cli.rs`
  - para que aparezca en la ayuda del CLI
- `riku_rust/README.md`
  - como linea visible para humanos

Tambien se aprovecho para corregir la edicion del crate de `2026` a `2024`.

## 12. Ejecucion del proyecto Rust

Se discutio como ejecutar `riku_rust`:

### Con cargo

```bash
cargo run -- diff <commit_a> <commit_b> <archivo.sch>
cargo run -- log <archivo.sch>
cargo run -- doctor
```

### Binario local

```bash
./target/debug/riku diff <commit_a> <commit_b> <archivo.sch>
```

En Windows:

```powershell
.\target\debug\riku.exe diff <commit_a> <commit_b> <archivo.sch>
```

### Binario instalado

```bash
cargo install --path .
riku diff <commit_a> <commit_b> <archivo.sch>
```

Se aclaro que esto es equivalente a la experiencia de uso que antes se tenia en Python con el comando `riku`.

## 13. Comparacion Python vs Rust

Se discutieron varias diferencias entre la version Python y la Rust.

### Diferencias importantes

- El cache key de render no es exactamente igual.
  - Python usaba `version + "::" + blob.hex()`
  - Rust usa `version + "::" + content_bytes`
- El parser de Xschem cambio de regex plano a parser por bloques y lineas.
- El diff semantico quedo reescrito con tipos fuertes y una semantica mas clara para cambios cosméticos.
- El render visual en Rust escribe `render.json` ademas del SVG.

### Lo que quedo alineado

- leer Git sin checkout
- detectar formato
- parsear Xschem
- hacer diff semantico
- renderizar SVG
- anotar cambios
- mostrar comandos de usuario equivalentes

La conclusion fue que Rust no copia literalmente a Python, pero si implementa la misma logica de negocio con mejor estructura.

## 14. Futuro del CLI

Se documento un archivo dedicado al futuro del CLI en:

- `planificacion/cli_futuro.md`

La idea principal fue:

- mantener `diff`, `log` y `doctor`
- agregar, en el futuro, un modo interactivo tipo consola
- permitir comandos como `/diff`, `/log`, `/doctor`, `/help`, `/quit`
- no duplicar la logica del core

## 15. Estado actual al cierre de la conversacion

Al cierre de la conversacion, el proyecto tenia:

- `riku_rust` compilando
- tests pasando
- README del crate Rust actualizado
- README principal con enlaces a la migracion
- planificacion documentada en Markdown
- `.gitignore` ajustado para Rust
- autor visible en metadata y documentacion

La direccion general quedo clara:

- Rust es la implementacion oficial
- Python queda como referencia historica y de paridad
- el CLI actual se mantiene estable
- el futuro del CLI se documenta antes de implementarlo
- `gdstk/rust` queda como integracion posterior

## 16. Resumen corto de decisiones mas importantes

- `riku_rust` sera el nucleo principal.
- La arquitectura es monolito modular con enfoque hexagonal.
- `edition = "2024"` es la correcta para el crate.
- `Cargo.lock` se mantiene versionado.
- `riku_rust/target/` no debe quedar en Git.
- El cache de render sigue basado en `SHA256`.
- El CLI futuro puede crecer a modo interactivo, pero sin romper el flujo actual.
