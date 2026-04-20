# Plan Maestro de Migración a Rust

## Resumen
Migrar `riku` desde Python a Rust en `C:\Users\ariel\Documents\riku_chip\riku_rust`, usando una arquitectura de **monolito modular + hexagonal** y estilo **clean code**. La migración se hará por fases, con paridad funcional primero y optimización/integración completa después. La integración con [`gdstk/rust`](C:\Users\ariel\Documents\riku_chip\gdstk\rust) quedará para el final.

## Fases

### Fase 1: Base del proyecto
- Crate Rust creado y compilando.
- Dependencias base definidas y actualizadas.
- Estructura modular inicial establecida.
- Reglas de implementación y estilo definidas.

### Fase 2: Tipos y contratos
- Migrar modelos de dominio.
- Definir traits/puertos para Git, parser, renderer y drivers.
- Mantener el núcleo independiente de CLI e infraestructura.

### Fase 3: Capa Git
- Implementar lectura de blobs, commits y cambios con `git2`.
- Mantener la regla de no modificar el working tree.

### Fase 4: Parser de Xschem
- Reescribir el parser de `.sch` en Rust.
- Detectar componentes, wires, nets y formato básico.
- Agregar tests con archivos reales.

### Fase 5: Diff semántico
- Migrar la comparación semántica.
- Detectar componentes y nets agregadas/eliminadas/modificadas.
- Identificar cambios cosméticos como `Move All`.

### Fase 6: CLI
- Implementar `diff`, `log` y `doctor`.
- Soportar salida texto, JSON y visual.

### Fase 7: Render y anotación visual
- Integrar `xschem` como herramienta externa.
- Portar la anotación SVG.
- Cachear renders y validar precisión de coordenadas.

### Fase 8: Paridad completa con Python
- Comparar resultados de Rust contra `riku/`.
- Ajustar diferencias de parseo, diff y visualización.
- Definir si Python queda como wrapper o legado.

### Fase 9: Integración con `gdstk/rust`
- Conectar `riku_rust` con [`gdstk/rust`](C:\Users\ariel\Documents\riku_chip\gdstk\rust).
- Definir si la integración será por workspace, dependencia local o API compartida.
- Evitar acoplar antes de cerrar la paridad principal.

## Dependencias actuales
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

## Criterios de aceptación
- `riku_rust` compila y pasa tests.
- `diff` y `log` funcionan con Xschem.
- El modo visual genera SVG anotado.
- La paridad funcional con Python se acerca o se alcanza.
- La integración con `gdstk/rust` queda preparada para la fase final.

## Suposiciones
- `riku_rust` es el núcleo principal.
- `xschem` sigue siendo dependencia externa para render visual.
- La migración es incremental.
- Se prioriza reproducibilidad y claridad por encima de reescritura agresiva.

