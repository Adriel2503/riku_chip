"""
Prueba del parser, diff semantico y render SVG.
Uso: python tests/test_xschem_parser.py <a.sch> [b.sch]
"""
import json
import sys
from pathlib import Path
from dataclasses import asdict

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.parsers.xschem import detect_format, parse
from riku.core.semantic_diff import diff
from riku.adapters.xschem_adapter import available, diff_visual


def main():
    if len(sys.argv) < 2:
        print("Uso: python tests/test_xschem_parser.py <a.sch> [b.sch]")
        sys.exit(1)

    path_a = Path(sys.argv[1])
    content_a = path_a.read_bytes()

    fmt = detect_format(content_a)
    print(f"Formato detectado: {fmt}")

    if fmt != "xschem":
        print("No es un archivo Xschem — fallback a diff de texto.")
        sys.exit(0)

    sch = parse(content_a)
    print(f"\n--- Esquematico: {path_a.name} ---")
    print(f"Componentes: {len(sch.components)}")
    for name, c in sch.components.items():
        print(f"  {name}: {c.symbol} | {c.params}")
    print(f"Nets: {sorted(sch.nets)}")
    print(f"Wires: {len(sch.wires)}")

    ok, version = available()
    print(f"\nXschem disponible: {ok} — {version}")

    if len(sys.argv) >= 3:
        path_b = Path(sys.argv[2])
        content_b = path_b.read_bytes()
        report = diff(content_a, content_b)

        print(f"\n--- Diff semantico: {path_a.name} -> {path_b.name} ---")
        if report.is_empty():
            print("Sin cambios semanticos.")
        if report.is_move_all:
            print("AVISO: reorganizacion cosmetica detectada (Move All).")
        for cd in report.components:
            if cd.kind == "added":
                print(f"  [+] {cd.name}: {cd.after}")
            elif cd.kind == "removed":
                print(f"  [-] {cd.name}: {cd.before}")
            elif cd.kind == "modified":
                print(f"  [~] {cd.name}: {cd.before} -> {cd.after}")
        for net in report.nets_added:
            print(f"  [+net] {net}")
        for net in report.nets_removed:
            print(f"  [-net] {net}")

        print("\n--- JSON ---")
        print(json.dumps(asdict(report), indent=2))

        if ok:
            print("\n--- Render SVG ---")
            svg_a, svg_b = diff_visual(path_a, path_b)
            if svg_a:
                print(f"  SVG A: {svg_a}")
            if svg_b:
                print(f"  SVG B: {svg_b}")
            if not svg_a and not svg_b:
                print("  Render fallido.")


if __name__ == "__main__":
    main()
