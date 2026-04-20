"""
Test del anotador SVG con datos reales de Xschem.
Uso: python tests/test_svg_annotator.py
"""
import sys
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.parsers.xschem import parse
from riku.core.models import DiffReport, ComponentDiff
from riku.core.svg_annotator import annotate, _extract_name_positions, _fit_transform

SCH_PATH = Path("/foss/designs/caravel_user_project_analog/xschem/example_por.sch")
SVG_PATH = Path("/tmp/example_por.svg")


def main():
    if not SCH_PATH.exists():
        print(f"[SKIP] {SCH_PATH} no encontrado — correr en Docker")
        return
    if not SVG_PATH.exists():
        print(f"[SKIP] {SVG_PATH} no encontrado — generar SVG primero")
        return

    schematic = parse(SCH_PATH.read_bytes())
    svg_content = SVG_PATH.read_text(encoding="utf-8")

    # --- paso 1: extraer posiciones del SVG ---
    positions = _extract_name_positions(svg_content)
    print(f"Nombres encontrados en SVG: {len(positions)}")
    for name, (x, y) in list(positions.items())[:5]:
        print(f"  {name:<6} svg=({x:.2f}, {y:.2f})")

    # --- paso 2: calcular transformacion ---
    transform = _fit_transform(positions, schematic)
    if transform is None:
        print("ERROR: no se pudo calcular transformacion")
        return

    print(f"\nTransformacion calculada:")
    print(f"  mooz    = {transform.mooz:.6f}")
    print(f"  offset_x = {transform.offset_x:.4f}")
    print(f"  offset_y = {transform.offset_y:.4f}")

    # --- verificar con componentes conocidos ---
    comp_names = list(schematic.components.keys())
    print(f"\nVerificacion de componentes:")
    for name in comp_names[:3]:
        if name in positions:
            comp = schematic.components[name]
            pred_x, pred_y = transform.to_svg(comp.x, comp.y)
            real_x, real_y = positions[name]
            err_x = abs(pred_x - real_x)
            err_y = abs(pred_y - real_y)
            print(f"  {name}: pred=({pred_x:.2f}, {pred_y:.2f}) real=({real_x:.2f}, {real_y:.2f}) err=({err_x:.3f}, {err_y:.3f})")

    # --- nets con wires en el esquematico ---
    nets_con_wires = list({w.label for w in schematic.wires if w.label and not w.label.startswith("#")})
    print(f"\nNets con label (no internas): {nets_con_wires[:5]}")

    # --- paso 3: anotar SVG con diff simulado ---
    c0 = comp_names[0] if len(comp_names) > 0 else "X"
    c1 = comp_names[1] if len(comp_names) > 1 else "X"
    c2 = comp_names[2] if len(comp_names) > 2 else "X"
    net_add  = nets_con_wires[0] if len(nets_con_wires) > 0 else ""
    net_rem  = nets_con_wires[1] if len(nets_con_wires) > 1 else ""

    fake_diff = DiffReport(
        components=[
            ComponentDiff(name=c0, kind="modified"),
            ComponentDiff(name=c1, kind="removed"),
            ComponentDiff(name=c2, kind="added"),
        ],
        nets_added=[net_add] if net_add else [],
        nets_removed=[net_rem] if net_rem else [],
    )

    annotated = annotate(svg_content, schematic, fake_diff)
    out_path = Path("/tmp/example_por_annotated.svg")
    out_path.write_text(annotated, encoding="utf-8")
    print(f"\nSVG anotado escrito en: {out_path}")
    print(f"Contiene riku-diff-annotations: {'riku-diff-annotations' in annotated}")


if __name__ == "__main__":
    main()
