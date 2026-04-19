from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from pathlib import Path


# ---------------------------------------------------------------------------
# Modelos de resultado compartidos por todos los drivers
# ---------------------------------------------------------------------------

@dataclass
class DiffEntry:
    """Un cambio atómico dentro de un archivo de diseño."""
    kind: str          # "added" | "removed" | "modified"
    element: str       # identificador del elemento (nombre de componente, celda, net, capa...)
    before: dict | None = None
    after: dict | None = None
    cosmetic: bool = False  # True si el cambio es solo visual (ej. Move All)


@dataclass
class DriverDiffReport:
    """
    Resultado estándar de cualquier diff entre dos revisiones.
    Todos los drivers deben retornar este tipo desde diff().
    """
    file_type: str                        # "xschem" | "magic" | "gds" | "spice" ...
    changes: list[DiffEntry] = field(default_factory=list)
    visual_a: Path | None = None          # SVG/PNG de la revisión A (si disponible)
    visual_b: Path | None = None          # SVG/PNG de la revisión B (si disponible)
    warnings: list[str] = field(default_factory=list)

    def is_empty(self) -> bool:
        return not any(c for c in self.changes if not c.cosmetic)

    def has_visuals(self) -> bool:
        return self.visual_a is not None or self.visual_b is not None


@dataclass
class DriverInfo:
    """Información sobre el driver y la herramienta que maneja."""
    name: str           # "xschem" | "klayout" | "magic" | "ngspice"
    available: bool
    version: str        # versión de la herramienta externa, "" si no disponible
    extensions: list[str] = field(default_factory=list)  # [".sch"] | [".gds", ".oas"] | [".mag"]


# ---------------------------------------------------------------------------
# Protocolo base — todo driver debe heredar de esta clase
# ---------------------------------------------------------------------------

class RikuDriver(ABC):
    """
    Protocolo base para drivers de herramientas EDA en Riku.

    Cómo implementar un driver nuevo:
    1. Crear riku/adapters/<herramienta>_adapter.py
    2. Crear una clase que herede de RikuDriver
    3. Implementar los tres métodos abstractos: info(), diff(), normalize()
    4. Registrar el driver en riku/core/registry.py

    Reglas:
    - info() nunca lanza excepción — si la herramienta no está, retorna available=False
    - diff() recibe bytes crudos — no rutas. El GitService ya extrajo el contenido.
    - normalize() retorna bytes — el caller decide qué hacer con ellos
    - Ningún método debe hacer git checkout ni modificar el working tree
    """

    @abstractmethod
    def info(self) -> DriverInfo:
        """
        Detecta si la herramienta está instalada y retorna su información.
        Debe ser rápido — se llama en cada invocación de riku doctor.
        Nunca lanza excepción.
        """
        ...

    @abstractmethod
    def diff(
        self,
        content_a: bytes,
        content_b: bytes,
        path_hint: str = "",
    ) -> DriverDiffReport:
        """
        Compara dos revisiones de un archivo y retorna un reporte estructurado.

        Args:
            content_a: contenido raw de la revisión anterior (blob git)
            content_b: contenido raw de la revisión actual (blob git)
            path_hint: nombre del archivo original — solo para mensajes de error,
                       nunca para leer del disco

        Returns:
            DriverDiffReport con los cambios encontrados.
            Si la herramienta no está disponible, retorna reporte vacío con warning.
        """
        ...

    @abstractmethod
    def normalize(self, content: bytes, path_hint: str = "") -> bytes:
        """
        Normaliza el contenido de un archivo antes de compararlo con git diff.
        Ejemplo: eliminar timestamps de .mag, canonicalizar coordenadas de .sch.

        Args:
            content: contenido raw del archivo
            path_hint: nombre del archivo original

        Returns:
            bytes normalizados. Si no hay nada que normalizar, retornar content sin cambios.
        """
        ...

    def render(self, content: bytes, path_hint: str = "") -> Path | None:
        """
        Genera una representación visual (SVG o PNG) del archivo.
        Opcional — por defecto retorna None (sin visual).
        Override en drivers que soporten render headless.
        """
        return None

    def can_handle(self, filename: str) -> bool:
        """
        Retorna True si este driver puede manejar el archivo dado su nombre.
        Usa las extensiones declaradas en info().
        """
        suffix = Path(filename).suffix.lower()
        return suffix in self.info().extensions
