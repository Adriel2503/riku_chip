"""
Benchmark 3: Semantic diff — costo por cantidad de componentes.

Mide:
- Tiempo de diff() en funcion de N componentes (detectar si hay O(N^2) oculto)
- Escenarios: sin cambios, 10% modificados, 50% reemplazados, Move All
- Costo de parseo vs costo de comparacion de dicts (separados)

Uso: python tests/bench_semantic_diff.py
"""
import sys
import time
import statistics
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.core.semantic_diff import diff
from riku.parsers.xschem import parse


def _make_sch(n: int, offset_x: float = 0.0, modify_ratio: float = 0.0, replace_ratio: float = 0.0) -> bytes:
    """
    Genera un .sch con N componentes.
    - offset_x: desplaza todos los componentes en X (Move All)
    - modify_ratio: fraccion de componentes con params distintos
    - replace_ratio: fraccion de componentes con nombre distinto (removed+added)
    """
    lines = ["v {xschem version=3.4.5 file_version=1.2}\n"]
    n_modify = int(n * modify_ratio)
    n_replace = int(n * replace_ratio)

    for i in range(n):
        x = (100 + (i % 50) * 200) + offset_x
        y = 100 + (i // 50) * 200
        if i < n_replace:
            name = f"REPLACED{i}"
        else:
            name = f"M{i}"
        w = "2" if i < n_modify else "1"
        lines.append(
            f"C {{sky130_fd_pr/nfet_01v8.sym}} {x} {y} 0 0 {{name={name}\n"
            f"L=0.15\nW={w}\nnf=1\n}}\n"
        )
    return "".join(lines).encode("utf-8")


def bench_scenario(label: str, content_a: bytes, content_b: bytes, n_runs: int = 20) -> dict:
    # Calentar
    _ = diff(content_a, content_b)

    times = []
    for _ in range(n_runs):
        t0 = time.perf_counter()
        report = diff(content_a, content_b)
        times.append(time.perf_counter() - t0)

    mean_ms = statistics.mean(times) * 1000
    p95_ms = sorted(times)[int(0.95 * n_runs)] * 1000
    return {"label": label, "mean_ms": mean_ms, "p95_ms": p95_ms}


def main():
    print("=== Benchmark 3: Semantic diff — escalabilidad ===\n")

    sizes = [10, 50, 100, 250, 500, 1000]

    scenarios = [
        ("sin cambios",    dict(offset_x=0,    modify_ratio=0.0,  replace_ratio=0.0)),
        ("Move All",       dict(offset_x=500,  modify_ratio=0.0,  replace_ratio=0.0)),
        ("10% modificado", dict(offset_x=0,    modify_ratio=0.1,  replace_ratio=0.0)),
        ("50% reemplazado",dict(offset_x=0,    modify_ratio=0.0,  replace_ratio=0.5)),
    ]

    for scenario_label, kwargs in scenarios:
        print(f"[{scenario_label}]\n")
        print(f"  {'N comps':>8} {'mean ms':>10} {'p95 ms':>10} {'us/comp':>10}")
        print("  " + "-" * 45)

        for n in sizes:
            content_a = _make_sch(n)
            content_b = _make_sch(n, **kwargs)
            n_runs = max(10, min(50, 500 // n))
            r = bench_scenario(scenario_label, content_a, content_b, n_runs)
            us_per = r["mean_ms"] * 1000 / n
            print(f"  {n:>8} {r['mean_ms']:>10.3f} {r['p95_ms']:>10.3f} {us_per:>10.2f}")
        print()

    # --- aislar costo de parseo vs costo de comparacion ---
    print("[Desglose: parseo vs comparacion de dicts]\n")
    print(f"  {'N comps':>8} {'parseo A ms':>12} {'parseo B ms':>12} {'diff dict ms':>14}")
    print("  " + "-" * 52)

    for n in [100, 500, 1000]:
        content_a = _make_sch(n)
        content_b = _make_sch(n, modify_ratio=0.1)

        n_runs = 20

        # Medir solo parseo
        t0 = time.perf_counter()
        for _ in range(n_runs):
            sch_a = parse(content_a)
        parse_a_ms = (time.perf_counter() - t0) / n_runs * 1000

        t0 = time.perf_counter()
        for _ in range(n_runs):
            sch_b = parse(content_b)
        parse_b_ms = (time.perf_counter() - t0) / n_runs * 1000

        # Medir diff completo y restar parseo
        t0 = time.perf_counter()
        for _ in range(n_runs):
            diff(content_a, content_b)
        total_ms = (time.perf_counter() - t0) / n_runs * 1000
        dict_ms = max(0.0, total_ms - parse_a_ms - parse_b_ms)

        print(f"  {n:>8} {parse_a_ms:>12.3f} {parse_b_ms:>12.3f} {dict_ms:>14.3f}")

    print("\n=== Fin benchmark semantic diff ===")


if __name__ == "__main__":
    main()
