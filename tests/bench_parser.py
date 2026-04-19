"""
Benchmark 2: Parser .sch — escalabilidad por tamano de archivo.

Mide:
- Tiempo de parse vs tamano del archivo (bytes)
- Tiempo vs cantidad de componentes extraidos
- Costo del regex MULTILINE|DOTALL en archivos grandes

Uso:
  Local:  python tests/bench_parser.py
  Docker: python tests/bench_parser.py --docker-paths

Los archivos de Docker se pasan como argumentos adicionales o se usan los defaults.
"""
import sys
import time
import statistics
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.parsers.xschem import parse


DOCKER_SCH_FILES = [
    "/foss/designs/kit/logic/inv.sch",
    "/foss/designs/gilbert/multiplicador_bueno_elmejor_18_11_2023.sch",
    "/foss/designs/caravel_user_project_analog/xschem/example_por.sch",
    "/foss/designs/caravel_user_project_analog/xschem/user_analog_project_wrapper.sch",
    "/foss/designs/OpenRAM/ciel/sky130/versions/e8294524e5f67c533c5d0c3afa0bcc5b2a5fa066/sky130A/libs.tech/xschem/mips_cpu/alu.sch",
]


def bench_file(path: Path, n: int = 30) -> dict:
    content = path.read_bytes()
    size_kb = len(content) / 1024

    # Calentar
    _ = parse(content)

    times = []
    for _ in range(n):
        t0 = time.perf_counter()
        result = parse(content)
        times.append(time.perf_counter() - t0)

    mean_ms = statistics.mean(times) * 1000
    p95_ms = sorted(times)[int(0.95 * n)] * 1000
    n_components = len(result.components)
    n_wires = len(result.wires)

    return {
        "name": path.name,
        "size_kb": size_kb,
        "n_components": n_components,
        "n_wires": n_wires,
        "mean_ms": mean_ms,
        "p95_ms": p95_ms,
    }


def _make_synthetic_sch(n_components: int) -> bytes:
    """Genera un .sch sintetico con N componentes para medir escalabilidad pura."""
    lines = ["v {xschem version=3.4.5 file_version=1.2}\n"]
    for i in range(n_components):
        x = 100 + (i % 50) * 200
        y = 100 + (i // 50) * 200
        lines.append(
            f"C {{sky130_fd_pr/nfet_01v8.sym}} {x} {y} 0 0 {{name=M{i}\n"
            f"L=0.15\nW=1\nnf=1\n}}\n"
        )
    return "".join(lines).encode("utf-8")


def main():
    print("=== Benchmark 2: Parser .sch — escalabilidad ===\n")

    # --- archivos reales de Docker ---
    real_files = [Path(p) for p in DOCKER_SCH_FILES if Path(p).exists()]
    if not real_files:
        print("[!] No se encontraron archivos .sch de Docker — corriendo solo con sinteticos.\n")
    else:
        print(f"[1] Archivos reales ({len(real_files)} encontrados)\n")
        print(f"  {'archivo':<50} {'KB':>8} {'comps':>6} {'wires':>6} {'mean ms':>9} {'p95 ms':>9}")
        print("  " + "-" * 95)
        for p in real_files:
            r = bench_file(p)
            print(
                f"  {r['name']:<50} {r['size_kb']:>8.1f} {r['n_components']:>6} "
                f"{r['n_wires']:>6} {r['mean_ms']:>9.3f} {r['p95_ms']:>9.3f}"
            )
        print()

    # --- sinteticos: escala de 10 a 2000 componentes ---
    print("[2] Sinteticos — escala de componentes (multiline attrs)\n")
    sizes = [10, 50, 100, 250, 500, 1000, 2000]
    print(f"  {'N comps':>8} {'KB generado':>12} {'mean ms':>10} {'p95 ms':>10} {'us/comp':>10}")
    print("  " + "-" * 55)
    for n in sizes:
        content = _make_synthetic_sch(n)
        size_kb = len(content) / 1024
        n_runs = max(10, min(100, 2000 // n))

        _ = parse(content)
        times = []
        for _ in range(n_runs):
            t0 = time.perf_counter()
            parse(content)
            times.append(time.perf_counter() - t0)

        mean_ms = statistics.mean(times) * 1000
        p95_ms = sorted(times)[int(0.95 * n_runs)] * 1000
        us_per_comp = mean_ms * 1000 / n if n > 0 else 0
        print(f"  {n:>8} {size_kb:>12.1f} {mean_ms:>10.3f} {p95_ms:>10.3f} {us_per_comp:>10.2f}")

    print("\n=== Fin benchmark parser ===")


if __name__ == "__main__":
    main()
