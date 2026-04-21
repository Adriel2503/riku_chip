# riku-gui

`riku-gui` es la interfaz de escritorio experimental de Riku para visualizar archivos GDS usando el motor local `gds-renderer`.

La GUI es de solo lectura: permite abrir un GDS, ver el render, hacer zoom, mover la vista y activar o desactivar capas. No edita el layout.

## Arquitectura

- `riku-gui` usa `eframe`/`egui` para la ventana nativa de escritorio.
- `gds-renderer` se enlaza por `path = "../gds-renderer"` y genera la escena renderizable.
- `gdstk-rs` se enlaza por `path = "../gdstk/rust"` y carga/parses archivos GDS.
- `riku` puede lanzar la GUI con el comando `riku gui`.

## Ejecutar desde el workspace

Desde la raiz del repositorio:

```powershell
cd C:\Users\ariel\Documents\riku_chip

$env:VCPKG_ROOT='C:\vcpkg'
$env:VCPKG_DEFAULT_TRIPLET='x64-windows'
$env:PATH='C:\vcpkg\installed\x64-windows\bin;' + $env:PATH

cargo run -p riku-gui
```

Con un archivo GDS:

```powershell
cargo run -p riku-gui -- ruta\al\archivo.gds
```

## Ejecutar desde el CLI `riku`

Desde la carpeta `riku`:

```powershell
cd C:\Users\ariel\Documents\riku_chip\riku

$env:VCPKG_ROOT='C:\vcpkg'
$env:VCPKG_DEFAULT_TRIPLET='x64-windows'
$env:PATH='C:\vcpkg\installed\x64-windows\bin;' + $env:PATH

cargo run -- gui
```

Con un archivo GDS:

```powershell
cargo run -- gui ..\ruta\al\archivo.gds
```

## Verificar compilacion

Desde la raiz del repositorio:

```powershell
cd C:\Users\ariel\Documents\riku_chip

$env:VCPKG_ROOT='C:\vcpkg'
$env:VCPKG_DEFAULT_TRIPLET='x64-windows'
$env:PATH='C:\vcpkg\installed\x64-windows\bin;' + $env:PATH

cargo check -p riku-gui
```

Para verificar el CLI:

```powershell
cd C:\Users\ariel\Documents\riku_chip\riku

$env:VCPKG_ROOT='C:\vcpkg'
$env:VCPKG_DEFAULT_TRIPLET='x64-windows'
$env:PATH='C:\vcpkg\installed\x64-windows\bin;' + $env:PATH

cargo check
```

## Problemas encontrados

### `STATUS_DLL_NOT_FOUND` al ejecutar en Windows

Ejemplo:

```text
error: process didn't exit successfully: `target\debug\riku-gui.exe`
(exit code: 0xc0000135, STATUS_DLL_NOT_FOUND)
```

Esto significa que el binario compilo, pero Windows no encontro una DLL necesaria en runtime.

La causa mas probable es que `gdstk-rs` carga dependencias nativas instaladas por `vcpkg`, pero la carpeta de DLLs no esta en `PATH`.

Solucion:

```powershell
$env:PATH='C:\vcpkg\installed\x64-windows\bin;' + $env:PATH
```

Si `vcpkg` esta instalado en otra ruta, ajustar `C:\vcpkg`.

### Variables necesarias para `gdstk-rs`

En Windows se debe indicar donde esta `vcpkg`:

```powershell
$env:VCPKG_ROOT='C:\vcpkg'
$env:VCPKG_DEFAULT_TRIPLET='x64-windows'
```

Sin esto, la compilacion puede fallar al resolver las librerias nativas de `gdstk`.

### Cambios por `egui`/`eframe 0.34.1`

La GUI usa:

```toml
eframe = "0.34.1"
```

Durante la integracion se corrigieron incompatibilidades con la API actual:

- `raw_scroll_delta` fue reemplazado por `smooth_scroll_delta`.
- El zoom mezclaba `f32` con `f64`; ahora se convierte el scroll a `f64`.
- `TopBottomPanel` y `SidePanel` fueron reemplazados por `egui::Panel`.
- `show` fue reemplazado por `show_inside`.
- `default_width` fue reemplazado por `default_size`.
- `allocate_ui_at_rect` fue reemplazado por `scope_builder`.

### Borrow checker en el arbol de proyecto

El panel de proyecto leia `selected_path` mientras una closure podia mutar `self` al abrir archivos.

Solucion aplicada: clonar `selected_path` antes de crear la closure que llama `open_path`.

### `BoundingBox` en `scene_painter`

`world_to_screen` recibe `BoundingBox` por valor, pero el match entregaba `&BoundingBox` para rectangulos.

Solucion aplicada: pasar `*rect_bbox` en las llamadas de rectangulos.

### Merge conflict en `run_shell`

Durante el rebase se combino el shell interactivo remoto con el comando local `gui`.

El cierre de `run_shell` quedo incompleto temporalmente. Se corrigio agregando:

```rust
Ok(())
```

al final de la funcion.

## Estado esperado

La verificacion esperada es:

```text
cargo check -p riku-gui
Finished `dev` profile
```

Y para `riku`:

```text
cargo check
Finished `dev` profile
```

