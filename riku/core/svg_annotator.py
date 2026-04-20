"""
Anotador de SVGs de Xschem.

Flujo:
1. Parsear el SVG para extraer posiciones de nombres de componentes.
2. Cruzar con el Schematic para calcular la transformacion .sch -> SVG.
3. Dibujar bounding boxes de color sobre componentes del DiffReport.
"""
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass

from riku.core.models import Schematic, DiffReport

_TEXT_RE = re.compile(
    r'<text[^>]+transform="translate\(([0-9.\-]+),\s*([0-9.\-]+)\)"[^>]*>\s*([^<]+?)\s*</text>'
)

# Color de nombre de componente en SVGs de Xschem — #cccccc
_COMPONENT_NAME_COLOR = "#cccccc"


@dataclass
class _Transform:
    mooz: float
    offset_x: float
    offset_y: float

    def to_svg(self, sch_x: float, sch_y: float) -> tuple[float, float]:
        return (
            sch_x * self.mooz + self.offset_x,
            sch_y * self.mooz + self.offset_y,
        )


def _extract_name_positions(svg_content: str) -> dict[str, tuple[float, float]]:
    """Extrae {nombre: (svg_x, svg_y)} de los textos de nombres de componentes."""
    positions: dict[str, tuple[float, float]] = {}
    for m in _TEXT_RE.finditer(svg_content):
        x, y, text = float(m.group(1)), float(m.group(2)), m.group(3).strip()
        # Verificar que el elemento es un nombre de componente por su color
        tag = m.group(0)
        if _COMPONENT_NAME_COLOR in tag:
            positions[text] = (x, y)
    return positions


def _fit_transform(
    svg_positions: dict[str, tuple[float, float]],
    schematic: Schematic,
) -> _Transform | None:
    """
    Calcula mooz, offset_x, offset_y usando minimos cuadrados sobre todos
    los componentes que aparecen tanto en el SVG como en el Schematic.
    Requiere al menos 2 puntos para resolver el sistema.
    """
    pairs = []
    for name, (svg_x, svg_y) in svg_positions.items():
        if name in schematic.components:
            comp = schematic.components[name]
            pairs.append((comp.x, comp.y, svg_x, svg_y))

    if len(pairs) < 2:
        return None

    # Minimos cuadrados para escala uniforme: svg = mooz * sch + offset
    # Separado por eje X e Y, luego promediamos mooz
    n = len(pairs)
    sum_sx = sum(p[0] for p in pairs)
    sum_sy = sum(p[1] for p in pairs)
    sum_vx = sum(p[2] for p in pairs)
    sum_vy = sum(p[3] for p in pairs)
    sum_sx2 = sum(p[0] ** 2 for p in pairs)
    sum_sy2 = sum(p[1] ** 2 for p in pairs)
    sum_sxvx = sum(p[0] * p[2] for p in pairs)
    sum_syvy = sum(p[1] * p[3] for p in pairs)

    denom_x = n * sum_sx2 - sum_sx ** 2
    denom_y = n * sum_sy2 - sum_sy ** 2

    if abs(denom_x) < 1e-9 or abs(denom_y) < 1e-9:
        return None

    mooz_x = (n * sum_sxvx - sum_sx * sum_vx) / denom_x
    mooz_y = (n * sum_syvy - sum_sy * sum_vy) / denom_y
    mooz = (mooz_x + mooz_y) / 2

    offset_x = (sum_vx - mooz * sum_sx) / n
    offset_y = (sum_vy - mooz * sum_sy) / n

    return _Transform(mooz=mooz, offset_x=offset_x, offset_y=offset_y)


_BBOX_HALF = 15  # mitad del lado del bounding box en unidades .sch

_COLORS = {
    "added":    ("rgba(0,200,0,0.25)",   "rgba(0,200,0,0.8)"),
    "removed":  ("rgba(200,0,0,0.25)",   "rgba(200,0,0,0.8)"),
    "modified": ("rgba(255,180,0,0.25)", "rgba(255,180,0,0.8)"),
}

_WIRE_STROKE = {
    "added":   "rgba(0,200,0,0.9)",
    "removed": "rgba(200,0,0,0.9)",
}
_WIRE_WIDTH = 2.5


def _wire_elements(wires, net_names: set[str], kind: str, transform: _Transform) -> list[str]:
    """Genera elementos <line> SVG para todos los wires que pertenecen a alguna net del set."""
    stroke = _WIRE_STROKE[kind]
    elements = []
    for w in wires:
        if w.label not in net_names:
            continue
        x1, y1 = transform.to_svg(w.x1, w.y1)
        x2, y2 = transform.to_svg(w.x2, w.y2)
        elements.append(
            f'<line x1="{x1:.2f}" y1="{y1:.2f}" x2="{x2:.2f}" y2="{y2:.2f}" '
            f'stroke="{stroke}" stroke-width="{_WIRE_WIDTH}" stroke-linecap="round"/>'
        )
    return elements


def annotate(
    svg_content: str,
    sch_b: Schematic,
    diff_report: DiffReport,
    sch_a: Schematic | None = None,
) -> str:
    """
    Recibe el SVG del commit B, los Schematics de ambos commits y el DiffReport.
    Retorna el SVG con:
    - Bounding boxes sobre componentes cambiados (added/removed/modified)
    - Trayectos de wires para nets añadidas (verde) y eliminadas (rojo)

    sch_a es opcional — si no se provee, las nets eliminadas no se dibujan.
    Si no se puede calcular la transformacion, retorna el SVG sin modificar.
    """
    svg_positions = _extract_name_positions(svg_content)
    transform = _fit_transform(svg_positions, sch_b)

    if transform is None:
        return svg_content

    elements = []
    half = _BBOX_HALF * transform.mooz

    # --- bounding boxes de componentes ---
    for cd in diff_report.components:
        # componentes added/modified usan sch_b; removed usan sch_a si disponible
        if cd.kind == "removed":
            source = sch_a if sch_a is not None else sch_b
        else:
            source = sch_b

        if cd.name not in source.components:
            continue
        comp = source.components[cd.name]
        cx, cy = transform.to_svg(comp.x, comp.y)
        fill, stroke = _COLORS.get(cd.kind, _COLORS["modified"])
        elements.append(
            f'<rect x="{cx - half:.2f}" y="{cy - half:.2f}" '
            f'width="{2*half:.2f}" height="{2*half:.2f}" '
            f'fill="{fill}" stroke="{stroke}" stroke-width="1.5" '
            f'rx="3" ry="3"/>'
        )

    # --- trayectos de wires de nets añadidas (en sch_b) ---
    if diff_report.nets_added:
        elements.extend(_wire_elements(
            sch_b.wires, set(diff_report.nets_added), "added", transform
        ))

    # --- trayectos de wires de nets eliminadas (en sch_a) ---
    if diff_report.nets_removed and sch_a is not None:
        elements.extend(_wire_elements(
            sch_a.wires, set(diff_report.nets_removed), "removed", transform
        ))

    if not elements:
        return svg_content

    annotation_layer = (
        '\n<g id="riku-diff-annotations">\n'
        + "\n".join(elements)
        + "\n</g>\n"
    )

    return svg_content.replace("</svg>", annotation_layer + "</svg>")
