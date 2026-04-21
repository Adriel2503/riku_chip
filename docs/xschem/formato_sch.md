# Formato .sch de Xschem

## Estructura del archivo

Un `.sch` de Xschem es texto plano. Las secciones relevantes:

```
v {xschem version=3.4.7 file_version=1.2}
G {}
S {}
E {}
C {resistor.sym} 100 200 0 0 {name=R1 value=10k}
C {capacitor.sym} 300 200 0 0 {name=C1 value=100n}
W 100 200 300 200
```

## Líneas relevantes para el parser

| Prefijo | Significado |
|---------|-------------|
| `C` | Componente (instancia de símbolo) |
| `W` | Wire (conexión entre dos puntos) |
| `v` | Versión del archivo |
| `N` | Net label |

## Componente: formato

```
C {symbol.sym} X Y rotation mirror {atributos}
```

- `X Y`: coordenadas del anchor del símbolo en unidades Xschem
- `rotation`: 0, 1, 2, 3 (múltiplos de 90°)
- `mirror`: 0 o 1
- `atributos`: `key=value` separados por espacios, dentro de `{}`

Los atributos multilinea usan `\` al final de línea:

```
C {nmos.sym} 100 200 0 0 {name=M1
value=sky130_fd_pr__nfet_01v8
W=1 L=0.15}
```

## Wire: formato

```
W x1 y1 x2 y2
```

Wires horizontales o verticales. Xschem no soporta wires diagonales.

## Detección de formato

El header siempre comienza con:
```
v {xschem version=
```

Si no está presente, el archivo es de otro formato (Qucs, KiCad, desconocido).

## Atributos importantes por tipo de componente

| Atributo | Uso |
|----------|-----|
| `name` | Identificador único del componente (R1, C1, M1, ...) |
| `value` | Valor del componente (10k, 100n, 1.8v, ...) |
| `model` | Modelo SPICE |
| `W`, `L` | Ancho y largo para transistores |
| `lab` | Label de net (en componentes de tipo net label) |

## Coordenadas

Las coordenadas están en unidades Xschem internas. La relación con el SVG renderizado es:

```
svg_x = (sch_x + xorigin) * mooz
svg_y = (sch_y + yorigin) * mooz
```

donde `xorigin`, `yorigin` se obtienen via TCL (`xschem get xorigin`) y `mooz` se calibra desde endpoints de wires SVG (paths `MxLy`).

Ver: `research/arquitectura/svg_annotator_coordenadas.md`
