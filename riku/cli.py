import sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

import typer
from typing import Optional
from pathlib import Path

app = typer.Typer(help="Riku - VCS semantico para diseno de chips.")


@app.command()
def diff(
    commit_a: str = typer.Argument(..., help="Commit base (mas antiguo)"),
    commit_b: str = typer.Argument(..., help="Commit destino (mas nuevo)"),
    file_path: str = typer.Argument(..., help="Ruta al archivo dentro del repo"),
    repo: str = typer.Option(".", "--repo", "-r", help="Ruta al repositorio Git"),
):
    """Muestra los cambios semanticos de un archivo entre dos commits."""
    from riku.core.analyzer import analyze_diff

    report = analyze_diff(repo, commit_a, commit_b, file_path)

    for w in report.warnings:
        typer.echo(f"[!] {w}", err=True)

    if report.is_empty():
        typer.echo("Sin cambios semanticos.")
        return

    typer.echo(f"Archivo: {file_path}  ({report.file_type})")
    typer.echo(f"Cambios: {len(report.changes)}")
    typer.echo("")

    for change in report.changes:
        cosmetic = "  [cosmetico]" if change.cosmetic else ""
        typer.echo(f"  {change.kind:<10} {change.element}{cosmetic}")


@app.command()
def log(
    file_path: Optional[str] = typer.Argument(None, help="Filtrar por archivo (opcional)"),
    repo: str = typer.Option(".", "--repo", "-r", help="Ruta al repositorio Git"),
    limit: int = typer.Option(20, "--limit", "-n", help="Maximo de commits a mostrar"),
):
    """Lista el historial de commits, opcionalmente filtrado por archivo."""
    from riku.core.git_service import GitService

    svc = GitService(repo)
    commits = svc.get_commits(file_path)[:limit]

    if not commits:
        typer.echo("Sin commits encontrados.")
        return

    for c in commits:
        typer.echo(f"{c.short_id}  {c.author:<20}  {c.message[:60]}")


@app.command()
def doctor(
    repo: str = typer.Option(".", "--repo", "-r", help="Ruta al repositorio Git"),
):
    """Verifica que herramientas EDA estan disponibles."""
    from riku.core.registry import get_drivers

    drivers = get_drivers()
    any_missing = False

    for driver in drivers:
        info = driver.info()
        status = "[ok]" if info.available else "[x]"
        version = f"  {info.version}" if info.available else "  no encontrado"
        typer.echo(f"  {status}  {info.name:<12}{version}")
        if not info.available:
            any_missing = True

    if any_missing:
        raise typer.Exit(code=1)


def main():
    app()
