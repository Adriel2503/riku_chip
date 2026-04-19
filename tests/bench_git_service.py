"""
Benchmark 1: GitService — throughput de extraccion de blobs via pygit2.

Mide:
- Tiempo por blob (primer acceso vs accesos repetidos al mismo commit)
- Throughput en extracciones consecutivas (simula riku log --semantic con N commits)
- Comparacion de get_commits() con y sin filtro por archivo

Uso: python tests/bench_git_service.py [--repo <path>]
"""
import sys
import time
import statistics
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.core.git_service import GitService


def bench(label: str, fn, n: int = 10):
    times = []
    for _ in range(n):
        t0 = time.perf_counter()
        result = fn()
        times.append(time.perf_counter() - t0)
    mean_ms = statistics.mean(times) * 1000
    p95_ms = sorted(times)[int(0.95 * n)] * 1000
    print(f"  {label:<45} mean={mean_ms:7.2f}ms  p95={p95_ms:7.2f}ms  n={n}")
    return result


def main():
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", default=".")
    args = parser.parse_args()

    svc = GitService(args.repo)

    print("=== Benchmark 1: GitService throughput ===\n")

    # --- get_commits sin filtro ---
    print("[1] get_commits()")
    commits = bench("get_commits() sin filtro", lambda: svc.get_commits(), n=20)
    print(f"      -> {len(commits)} commits totales\n")

    if not commits:
        print("No hay commits — abortar.")
        return

    # --- get_commits con filtro por archivo ---
    # Buscar un archivo que tenga varios commits
    candidates = [
        "riku/adapters/xschem_driver.py",
        "riku/parsers/xschem.py",
        "riku/core/git_service.py",
        "riku/cli.py",
    ]
    target_file = None
    target_commits = []
    for f in candidates:
        fc = svc.get_commits(f)
        if len(fc) >= 2:
            target_file = f
            target_commits = fc
            break

    if target_file:
        print(f"[2] get_commits(file='{target_file}')  [{len(target_commits)} commits]")
        bench(f"get_commits('{target_file}')", lambda: svc.get_commits(target_file), n=20)
        print()

    # --- get_blob: primer acceso ---
    commit_oid = commits[0].oid
    blob_file = target_file or "riku/cli.py"

    # Calentar una vez fuera del benchmark para separar cache de FS vs pygit2
    try:
        _ = svc.get_blob(commit_oid, blob_file)
    except KeyError:
        blob_file = "riku/cli.py"
        _ = svc.get_blob(commit_oid, blob_file)

    print(f"[3] get_blob() — mismo commit, mismo archivo ({blob_file})")
    blob = bench(
        f"get_blob({commits[0].short_id}, '{blob_file}')",
        lambda: svc.get_blob(commit_oid, blob_file),
        n=50,
    )
    print(f"      -> {len(blob)} bytes\n")

    # --- get_blob: barrido de N commits distintos (simula log --semantic) ---
    n_sweep = min(len(commits), 20)
    sweep_commits = commits[:n_sweep]

    print(f"[4] get_blob() — barrido de {n_sweep} commits distintos (simula log --semantic)")
    t0 = time.perf_counter()
    sizes = []
    for c in sweep_commits:
        try:
            b = svc.get_blob(c.oid, blob_file)
            sizes.append(len(b))
        except KeyError:
            sizes.append(0)
    elapsed_ms = (time.perf_counter() - t0) * 1000
    mean_ms = elapsed_ms / n_sweep
    total_bytes = sum(sizes)
    print(f"  {'barrido ' + str(n_sweep) + ' commits':<45} total={elapsed_ms:.1f}ms  mean={mean_ms:.2f}ms/blob")
    print(f"      -> {total_bytes} bytes extraidos en total\n")

    # --- get_changed_files ---
    if len(commits) >= 2:
        print("[5] get_changed_files(commit_a, commit_b)")
        bench(
            f"get_changed_files({commits[1].short_id}, {commits[0].short_id})",
            lambda: svc.get_changed_files(commits[1].oid, commits[0].oid),
            n=30,
        )
        print()

    print("=== Fin benchmark GitService ===")


if __name__ == "__main__":
    main()
