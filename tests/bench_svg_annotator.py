"""
Benchmark 6: SVG annotator — escala con cantidad de componentes en diff.

Mide:
- _extract_name_positions(): costo del regex sobre SVG de tamano real
- _fit_transform(): costo de minimos cuadrados con N puntos de anclaje
- annotate() end-to-end: con 1, 10, 50, 100, 200 componentes cambiados

Puede correr sin Docker (usa SVGs sinteticos) o con SVGs reales de Xschem.

Uso:
  Local:  python tests/bench_svg_annotator.py
  Docker: python tests/bench_svg_annotator.py --svg /tmp/gilbert2.svg --sch /foss/designs/gilbert/multiplicador_bueno_elmejor_18_11_2023.sch
"""
import sys
import time
import argparse
import statistics
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.core.svg_annotator import annotate, _extract_name_positions, _fit_transform
from riku.core.models import Schematic, Component, DiffReport, ComponentDiff


def _make_synthetic_svg(n_components: int) -> str:
    """Genera un SVG sintetico con N elementos text de nombre de componente."""
    elements = ['<svg width="900" height="532" xmlns="http://www.w3.org/2000/svg">']
    for i in range(n_components):
        x = 50 + (i % 30) * 28
        y = 50 + (i // 30) * 28
        elements.append(
            f'<text fill="#cccccc" transform="translate({x},{y})">M{i}</text>'
        )
    elements.append("</svg>")
    return "\n".join(elements)


def _make_synthetic_schematic(n_components: int, svg_positions: dict) -> Schematic:
    """Crea un Schematic con coordenadas que corresponden al SVG (mooz=0.674, offset=50)."""
    sch = Schematic()
    mooz = 0.674
    for name, (svg_x, svg_y) in svg_positions.items():
        sch_x = (svg_x - 50) / mooz
        sch_y = (svg_y - 50) / mooz
        sch.components[name] = Component(
            name=name, symbol="sky130_fd_pr/nfet_01v8.sym",
            params={}, x=sch_x, y=sch_y, rotation=0, mirror=0
        )
    return sch


def bench(label: str, fn, n: int = 50):
    _ = fn()  # calentar
    times = []
    for _ in range(n):
        t0 = time.perf_counter()
        result = fn()
        times.append(time.perf_counter() - t0)
    mean_ms = statistics.mean(times) * 1000
    p95_ms = sorted(times)[int(0.95 * n)] * 1000
    print(f"  {label:<50} mean={mean_ms:8.3f}ms  p95={p95_ms:8.3f}ms")
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--svg", default=None, help="SVG real de Xschem")
    parser.add_argument("--sch", default=None, help=".sch real de Xschem")
    args = parser.parse_args()

    print("=== Benchmark 6: SVG annotator — escalabilidad ===\n")

    # --- con archivos reales si estan disponibles ---
    if args.svg and Path(args.svg).exists() and args.sch and Path(args.sch).exists():
        from riku.parsers.xschem import parse as parse_sch
        print(f"[Archivos reales] svg={args.svg}  sch={args.sch}\n")

        svg_content = Path(args.svg).read_text(encoding="utf-8")
        schematic = parse_sch(Path(args.sch).read_bytes())

        bench("_extract_name_positions (SVG real)", lambda: _extract_name_positions(svg_content))
        positions = _extract_name_positions(svg_content)
        bench("_fit_transform (SVG real)", lambda: _fit_transform(positions, schematic))
        print()

        # annotate con distintas cantidades de componentes en el diff
        comp_names = list(schematic.components.keys())
        print(f"  {'n cambiados':>12} {'mean ms':>10} {'p95 ms':>10}")
        print("  " + "-" * 38)
        for n_changed in [1, 5, 10, 25, 50, len(comp_names)]:
            if n_changed > len(comp_names):
                continue
            sample = comp_names[:n_changed]
            fake_diff = DiffReport(
                components=[ComponentDiff(name=n, kind="modified") for n in sample]
            )
            times = []
            for _ in range(30):
                t0 = time.perf_counter()
                annotate(svg_content, schematic, fake_diff)
                times.append(time.perf_counter() - t0)
            mean_ms = statistics.mean(times) * 1000
            p95_ms = sorted(times)[int(0.95 * 30)] * 1000
            print(f"  {n_changed:>12} {mean_ms:>10.3f} {p95_ms:>10.3f}")
        print()

    # --- sinteticos: siempre se ejecutan ---
    print("[Sinteticos — distintos N de componentes en el SVG]\n")

    svg_sizes = [10, 50, 100, 250, 500, 1000]
    print(f"  {'N comps SVG':>12} {'extract ms':>12} {'fit_transform ms':>17} {'annotate(N) ms':>16}")
    print("  " + "-" * 62)

    for n in svg_sizes:
        svg = _make_synthetic_svg(n)
        positions = _extract_name_positions(svg)
        sch = _make_synthetic_schematic(n, positions)
        fake_diff = DiffReport(
            components=[ComponentDiff(name=f"M{i}", kind="modified") for i in range(n)]
        )

        n_runs = max(10, min(100, 1000 // n))

        # extract
        t0 = time.perf_counter()
        for _ in range(n_runs): _extract_name_positions(svg)
        extract_ms = (time.perf_counter() - t0) / n_runs * 1000

        # fit_transform
        t0 = time.perf_counter()
        for _ in range(n_runs): _fit_transform(positions, sch)
        fit_ms = (time.perf_counter() - t0) / n_runs * 1000

        # annotate
        t0 = time.perf_counter()
        for _ in range(n_runs): annotate(svg, sch, fake_diff)
        ann_ms = (time.perf_counter() - t0) / n_runs * 1000

        print(f"  {n:>12} {extract_ms:>12.3f} {fit_ms:>17.3f} {ann_ms:>16.3f}")

    print("\n=== Fin benchmark SVG annotator ===")


if __name__ == "__main__":
    main()
