"""
Benchmark 5: riku log --semantic end-to-end.

Simula un repo Git con N commits sobre un .sch y mide el tiempo total
de `riku log --semantic`, que es el caso de uso mas costoso:
  N extracciones de blob x parseo x diff = N * (get_blob + parse + parse + diff)

Usa un repo Git temporal en /tmp para no contaminar el repo real.

Uso: python tests/bench_log_semantic.py [--commits 20] [--components 50]
"""
import sys
import time
import argparse
import subprocess
import tempfile
import statistics
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from riku.core.git_service import GitService
from riku.core.analyzer import analyze_diff


def _make_sch(n_components: int, version: int = 0) -> bytes:
    """Genera un .sch con variaciones entre versiones para producir diffs reales."""
    lines = ["v {xschem version=3.4.5 file_version=1.2}\n"]
    for i in range(n_components):
        x = 100 + (i % 50) * 200
        y = 100 + (i // 50) * 200
        # Cada version modifica ~10% de componentes (W cambia)
        w = str(1 + ((i + version) % 3))
        lines.append(
            f"C {{sky130_fd_pr/nfet_01v8.sym}} {x} {y} 0 0 {{name=M{i}\n"
            f"L=0.15\nW={w}\nnf=1\n}}\n"
        )
    return "".join(lines).encode("utf-8")


def _init_test_repo(tmp_dir: Path, n_commits: int, n_components: int) -> tuple[Path, str]:
    """Crea un repo Git temporal con N commits sobre design.sch."""
    repo_dir = tmp_dir / "bench_repo"
    repo_dir.mkdir()

    env = {"GIT_AUTHOR_NAME": "bench", "GIT_AUTHOR_EMAIL": "bench@riku",
           "GIT_COMMITTER_NAME": "bench", "GIT_COMMITTER_EMAIL": "bench@riku",
           "PATH": subprocess.os.environ.get("PATH", "")}

    subprocess.run(["git", "init"], cwd=repo_dir, capture_output=True, env=env)
    subprocess.run(["git", "config", "user.email", "bench@riku"], cwd=repo_dir, capture_output=True)
    subprocess.run(["git", "config", "user.name", "bench"], cwd=repo_dir, capture_output=True)

    sch_path = repo_dir / "design.sch"
    for v in range(n_commits):
        sch_path.write_bytes(_make_sch(n_components, version=v))
        subprocess.run(["git", "add", "design.sch"], cwd=repo_dir, capture_output=True)
        subprocess.run(["git", "commit", "-m", f"version {v}"], cwd=repo_dir,
                       capture_output=True, env=env)

    return repo_dir, "design.sch"


def bench_log_semantic(repo_dir: Path, file_path: str, limit: int) -> dict:
    """Replica exactamente lo que hace `riku log --semantic --limit N`."""
    svc = GitService(str(repo_dir))
    commits = svc.get_commits(file_path)[:limit]

    if len(commits) < 2:
        return {}

    t0 = time.perf_counter()
    results = []
    for i, c in enumerate(commits):
        if i + 1 < len(commits):
            try:
                report = analyze_diff(str(repo_dir), commits[i + 1].oid, c.oid, file_path)
                added   = sum(1 for ch in report.changes if ch.kind == "added"    and not ch.cosmetic)
                removed = sum(1 for ch in report.changes if ch.kind == "removed"  and not ch.cosmetic)
                modified= sum(1 for ch in report.changes if ch.kind == "modified" and not ch.cosmetic)
                results.append((added, removed, modified))
            except Exception:
                pass

    elapsed_ms = (time.perf_counter() - t0) * 1000
    n_diffs = len(results)

    return {
        "elapsed_ms": elapsed_ms,
        "n_diffs": n_diffs,
        "ms_per_diff": elapsed_ms / n_diffs if n_diffs > 0 else 0,
        "sample": results[:3],
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--commits", type=int, default=20, help="Numero de commits a crear")
    parser.add_argument("--components", type=int, default=50, help="Componentes por .sch")
    args = parser.parse_args()

    print("=== Benchmark 5: riku log --semantic end-to-end ===\n")
    print(f"Configuracion: {args.commits} commits, {args.components} componentes por version\n")

    with tempfile.TemporaryDirectory(prefix="riku_bench_") as tmp:
        tmp_path = Path(tmp)

        # Crear repo
        print("Creando repo temporal con commits...")
        t0 = time.perf_counter()
        repo_dir, file_path = _init_test_repo(tmp_path, args.commits, args.components)
        setup_ms = (time.perf_counter() - t0) * 1000
        print(f"  Repo creado en {setup_ms:.0f}ms ({args.commits} commits)\n")

        # Benchmark principal
        print("[1] log --semantic completo\n")
        limits = [5, 10, 20]
        if args.commits >= 50:
            limits.append(50)

        print(f"  {'limit':>8} {'total ms':>10} {'n diffs':>8} {'ms/diff':>10}")
        print("  " + "-" * 42)

        for limit in limits:
            if limit > args.commits:
                continue
            r = bench_log_semantic(repo_dir, file_path, limit)
            if r:
                print(f"  {limit:>8} {r['elapsed_ms']:>10.1f} {r['n_diffs']:>8} {r['ms_per_diff']:>10.2f}")

        print()

        # Benchmark con distintos tamanios de esquematico
        print("[2] Impacto del tamano del esquematico (limit=10)\n")
        comp_sizes = [10, 50, 100, 250, 500]
        print(f"  {'comps':>8} {'total ms':>10} {'ms/diff':>10}")
        print("  " + "-" * 35)

        for n_comp in comp_sizes:
            sub_tmp = tmp_path / f"repo_{n_comp}"
            sub_tmp.mkdir()
            rd, fp = _init_test_repo(sub_tmp, 12, n_comp)
            r = bench_log_semantic(rd, fp, 10)
            if r:
                print(f"  {n_comp:>8} {r['elapsed_ms']:>10.1f} {r['ms_per_diff']:>10.2f}")

    print("\n=== Fin benchmark log --semantic ===")


if __name__ == "__main__":
    main()
