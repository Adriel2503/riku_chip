import subprocess
import hashlib
from pathlib import Path


CACHE_DIR = Path.home() / ".cache" / "riku" / "ops"

def _find_xschem() -> str | None:
    import shutil
    return shutil.which("xschem")


def available() -> tuple[bool, str]:
    """Retorna (disponible, version_string)."""
    xschem = _find_xschem()
    if not xschem:
        return False, ""
    try:
        result = subprocess.run(
            [xschem, "--version"],
            capture_output=True, text=True, timeout=10
        )
        for line in (result.stdout + result.stderr).splitlines():
            if "XSCHEM V" in line:
                return True, line.strip()
        return True, "unknown"
    except Exception:
        return False, ""


def _cache_key(sch_path: Path, xschem_version: str) -> str:
    blob = sch_path.read_bytes()
    raw = f"{xschem_version}::{blob.hex()}"
    return hashlib.sha256(raw.encode()).hexdigest()


def render_svg(sch_path: Path) -> Path | None:
    """
    Genera un SVG del esquemático usando Xschem headless.
    Usa el método Tcl (compatible con iic-osic-tools y Xschem >= 3.1).
    Cachea el resultado por contenido + version de Xschem.
    """
    ok, version = available()
    if not ok:
        return None

    xschem = _find_xschem()
    sch_path = sch_path.resolve()
    key = _cache_key(sch_path, version)
    cached = CACHE_DIR / key / "render.svg"

    if cached.exists():
        return cached

    cached.parent.mkdir(parents=True, exist_ok=True)

    try:
        cmd = (
            f"{xschem} "
            f'--tcl "wm iconify ." '
            f'--command "xschem zoom_full; xschem toggle_colorscheme; xschem print svg {cached}" '
            f'--quit {sch_path}'
        )
        subprocess.run(cmd, shell=True, capture_output=True, timeout=30)
        if cached.exists():
            return cached
        return None
    except Exception:
        return None


def diff_visual(sch_a: Path, sch_b: Path) -> tuple[Path | None, Path | None]:
    """Genera SVGs de dos revisiones para comparación side-by-side."""
    svg_a = render_svg(sch_a)
    svg_b = render_svg(sch_b)
    return svg_a, svg_b
