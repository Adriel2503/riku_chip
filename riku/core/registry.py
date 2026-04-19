from riku.core.driver import RikuDriver


# Registro central de drivers disponibles.
# Cada driver se importa aquí y se añade a la lista.
# El orden importa: el primero que retorne can_handle=True gana.

def get_drivers() -> list[RikuDriver]:
    from riku.adapters.xschem_driver import XschemDriver
    # from riku.adapters.klayout_driver import KLayoutDriver   # pendiente
    # from riku.adapters.magic_driver import MagicDriver       # pendiente
    # from riku.adapters.spice_driver import SpiceDriver       # pendiente

    return [
        XschemDriver(),
        # KLayoutDriver(),
        # MagicDriver(),
        # SpiceDriver(),
    ]


def get_driver_for(filename: str) -> RikuDriver | None:
    """Retorna el primer driver capaz de manejar el archivo, o None."""
    for driver in get_drivers():
        if driver.can_handle(filename):
            return driver
    return None
