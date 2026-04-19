from riku.core.models import ComponentDiff, DiffReport
from riku.parsers.xschem import parse


def diff(content_a: bytes, content_b: bytes) -> DiffReport:
    sch_a = parse(content_a)
    sch_b = parse(content_b)
    report = DiffReport()

    names_a = set(sch_a.components)
    names_b = set(sch_b.components)

    # Componentes eliminados
    for name in names_a - names_b:
        report.components.append(ComponentDiff(
            name=name, kind="removed",
            before=sch_a.components[name].params,
        ))

    # Componentes añadidos
    for name in names_b - names_a:
        report.components.append(ComponentDiff(
            name=name, kind="added",
            after=sch_b.components[name].params,
        ))

    # Componentes modificados (ignorar coordenadas)
    coord_only_changes = 0
    for name in names_a & names_b:
        ca, cb = sch_a.components[name], sch_b.components[name]
        coords_changed = (ca.x, ca.y, ca.rotation, ca.mirror) != (cb.x, cb.y, cb.rotation, cb.mirror)
        params_changed = ca.params != cb.params or ca.symbol != cb.symbol

        if params_changed:
            report.components.append(ComponentDiff(
                name=name, kind="modified",
                before={"symbol": ca.symbol, **ca.params},
                after={"symbol": cb.symbol, **cb.params},
            ))
        elif coords_changed:
            coord_only_changes += 1

    # Detectar "Move All": > 80% de componentes comunes solo cambiaron coordenadas
    common = len(names_a & names_b)
    if common > 0 and coord_only_changes / common > 0.8:
        report.is_move_all = True

    # Nets
    report.nets_added = sorted(sch_b.nets - sch_a.nets)
    report.nets_removed = sorted(sch_a.nets - sch_b.nets)

    return report
