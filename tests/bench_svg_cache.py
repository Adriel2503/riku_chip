"""
Benchmark 4: Cache SVG — hit vs miss.

Mide:
- Cache miss: tiempo de render completo via xschem (subprocess + zoom_full + SVG export)
- Cache hit: tiempo de retornar Path desde disco (~acceso a filesystem)
- Razon de speedup hit/miss

Requiere: xschem instalado (Docker o local con xschem en PATH).
Uso:
  Docker: docker exec -e PYTHONPATH=//foss/designs/riku ... python tests/bench_svg_cache.py
  Local:  python tests/bench_svg_cache.py  (skip si xschem no esta en PATH)
"""
import sys
import time
import shutil
import statistics
import tempfile
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.adapters.xschem_driver import XschemDriver, CACHE_DIR

REAL_SCH_FILES = [
    "/foss/designs/kit/logic/inv.sch",
    "/foss/designs/gilbert/multiplicador_bueno_elmejor_18_11_2023.sch",
]


def main():
    print("=== Benchmark 4: Cache SVG hit vs miss ===\n")

    driver = XschemDriver()
    info = driver.info()

    if not info.available:
        print("[SKIP] xschem no disponible — correr en Docker con sak-pdk sky130A.")
        return

    print(f"xschem: {info.version}\n")

    sch_files = [Path(p) for p in REAL_SCH_FILES if Path(p).exists()]
    if not sch_files:
        print("[SKIP] No se encontraron archivos .sch. Usar en Docker.")
        return

    print(f"  {'archivo':<48} {'miss ms':>10} {'hit ms':>10} {'speedup':>10}")
    print("  " + "-" * 84)

    for sch_path in sch_files:
        content = sch_path.read_bytes()

        import hashlib
        key = hashlib.sha256(info.version.encode() + b"::" + content).hexdigest()
        cached = CACHE_DIR / key / "render.svg"

        # Forzar cache miss eliminando el archivo si existe
        if cached.exists():
            cached.unlink()

        # --- cache miss ---
        t0 = time.perf_counter()
        result = driver.render(content, str(sch_path))
        miss_ms = (time.perf_counter() - t0) * 1000

        if result is None:
            print(f"  {sch_path.name:<48} {'RENDER FALLIDO':>10}")
            continue

        # --- cache hit (5 repeticiones para estabilizar) ---
        hit_times = []
        for _ in range(5):
            t0 = time.perf_counter()
            driver.render(content, str(sch_path))
            hit_times.append((time.perf_counter() - t0) * 1000)

        hit_ms = statistics.mean(hit_times)
        speedup = miss_ms / hit_ms if hit_ms > 0 else float("inf")

        svg_size_kb = result.stat().st_size / 1024
        print(
            f"  {sch_path.name:<48} {miss_ms:>10.1f} {hit_ms:>10.3f} {speedup:>9.0f}x"
            f"  (SVG {svg_size_kb:.0f} KB)"
        )

    # --- latencia pura de acceso a disco (Path.exists + stat) ---
    print()
    print("[Referencia: latencia de acceso a disco puro]\n")
    if sch_files:
        content = sch_files[0].read_bytes()
        import hashlib
        key = hashlib.sha256(info.version.encode() + b"::" + content).hexdigest()
        cached = CACHE_DIR / key / "render.svg"

        if cached.exists():
            times = []
            for _ in range(100):
                t0 = time.perf_counter()
                _ = cached.exists()
                times.append((time.perf_counter() - t0) * 1000)
            print(f"  Path.exists() mean={statistics.mean(times)*1000:.3f}us  p95={sorted(times)[94]*1000:.3f}us")

    print("\n=== Fin benchmark cache SVG ===")


if __name__ == "__main__":
    main()
