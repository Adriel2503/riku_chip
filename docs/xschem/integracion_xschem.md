# Integración con Xschem

## Cómo riku invoca Xschem

Xschem se invoca como proceso externo. No hay API embebida.

### Render a SVG

```bash
xschem --tcl "wm iconify ." \
       --command "xschem zoom_full; set _f [open $env(RIKU_ORIGINS_PATH) w]; puts $_f [xschem get xorigin]; puts $_f [xschem get yorigin]; close $_f; xschem print svg /ruta/render.svg" \
       --quit \
       /ruta/archivo.sch
```

**Puntos críticos:**
- `--tcl` recibe código TCL inline, no un archivo
- `$env(RIKU_ORIGINS_PATH)` es interpretado por TCL, no por bash — sin `shell=True` en Python o sin pasar por bash en Rust, el `$` llega intacto a TCL
- `wm iconify .` oculta la ventana Tk antes de que se dibuje
- `--quit` termina Xschem al finalizar el comando

### Detección de versión

```bash
xschem --version
```

La versión aparece en stdout o stderr con formato `XSCHEM V3.4.7`.

### Variables de entorno necesarias

| Variable | Propósito |
|----------|-----------|
| `RIKU_ORIGINS_PATH` | Ruta donde TCL escribe xorigin y yorigin |
| `PATH` | Debe incluir directorio de xschem |

En Docker (iic-osic-tools), xschem está en `/foss/tools/bin/`. El PATH minimal de `docker exec` no lo incluye — requiere activar el entorno primero:
```bash
docker exec -it <container> bash -c "PATH=/foss/tools/bin:$PATH riku doctor"
```

## Caché de renders

El render SVG se cachea para evitar re-invocar Xschem en cada diff.

**Clave de caché:** `SHA256(version_xschem + "::" + contenido_sch)`

**Estructura en disco:**
```
~/.cache/riku/ops/<sha256>/
    render.svg      ← el SVG generado
    origins.txt     ← xorigin\nyorigin (escrito por TCL)
    render.json     ← manifest con driver, version, source_sha256
```

**Lógica:**
1. Calcular clave SHA256
2. Si `render.svg` ya existe → retornar inmediatamente (cache hit)
3. Si no → invocar Xschem, escribir SVG y origins.txt

**Speedup medido:** ~3500x en cache hit vs primer render.

## origins.txt

Contiene exactamente dos líneas:
```
-123.456
789.012
```

Línea 1: `xorigin` (coordenada X del origen del viewport)
Línea 2: `yorigin` (coordenada Y del origen del viewport)

Estos valores son necesarios para la calibración exacta de `mooz`.

## Problemas conocidos

Ver: `research/arquitectura/gotchas_xschem.md`

Los más importantes:
- `toggle_colorscheme` ya no existe en Xschem ≥3.4 — no llamarlo
- Cada componente genera **dos** textos en el SVG (nombre + valor) con color `#cccccc`; el parser solo usa el primero como anchor
- Los paths de wires usan espacios: `M770.221 404.358L779.251 404.358` — no comas
- Sin `origins.txt`, el `mooz` estimado desde textos tiene ~0.5% de bias → ~5-10px de offset en wires

## Xschem en entornos sin display

Xschem necesita un display X11 o Wayland para inicializarse, incluso en modo headless.

En Linux/Docker sin pantalla:
```bash
Xvfb :99 -screen 0 1280x1024x24 &
export DISPLAY=:99
```

El contenedor `iic-osic-tools` ya configura esto automáticamente cuando se usa con VNC.
