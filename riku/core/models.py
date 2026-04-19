from dataclasses import dataclass, field


@dataclass
class Component:
    name: str
    symbol: str
    params: dict
    x: float
    y: float
    rotation: int = 0
    mirror: int = 0


@dataclass
class Wire:
    x1: float
    y1: float
    x2: float
    y2: float
    label: str = ""


@dataclass
class Schematic:
    components: dict[str, Component] = field(default_factory=dict)
    wires: list[Wire] = field(default_factory=list)
    nets: set[str] = field(default_factory=set)


@dataclass
class ComponentDiff:
    name: str
    kind: str  # "added" | "removed" | "modified"
    before: dict | None = None
    after: dict | None = None


@dataclass
class DiffReport:
    components: list[ComponentDiff] = field(default_factory=list)
    nets_added: list[str] = field(default_factory=list)
    nets_removed: list[str] = field(default_factory=list)
    is_move_all: bool = False

    def is_empty(self) -> bool:
        return (
            not self.components
            and not self.nets_added
            and not self.nets_removed
        )
