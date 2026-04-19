# Estructura del Proyecto

```text
riku/
├── riku/                       # paquete Python (pip install riku)
│   ├── cli.py                  # entry point: riku diff, riku log, riku doctor
│   ├── core/                   # lógica pura — sin imports de herramientas EDA
│   │   ├── driver.py           # protocolo RikuDriver (ABC)
│   │   ├── git_service.py      # extracción de blobs via pygit2
│   │   ├── cache.py            # caché L1 content-addressable (SQLite + filesystem)
│   │   └── models.py           # DiffReport, Component, Wire, Net
│   ├── parsers/                # parsers de formatos EDA (sin dependencias externas)
│   │   └── xschem.py           # parser .sch → objetos, SemanticDiff
│   └── adapters/               # drivers que hablan con herramientas externas
│       ├── xschem_adapter.py   # render SVG headless, xschem --diff
│       └── klayout_adapter.py  # XOR geométrico, LayoutDiff, DRC batch
├── tests/
│   ├── fixtures/               # archivos .sch reales para tests
│   ├── test_git_service.py
│   ├── test_xschem_parser.py
│   └── test_semantic_diff.py
├── pyproject.toml
└── miku.toml.example           # configuración de proyecto de ejemplo
```

---

## Separación de responsabilidades

| Módulo | Puede importar | No puede importar |
|---|---|---|
| `core/` | stdlib, pygit2, modelos propios | klayout, xschem, adapters |
| `parsers/` | stdlib, modelos propios | klayout, xschem, adapters, core |
| `adapters/` | core, parsers, herramientas EDA | ui |
| `cli.py` | core, parsers, adapters | ui directamente |
| `ui/` | core, parsers, adapters | nunca bloquear el main thread |

La regla clave: `core/` y `parsers/` son testeables sin ninguna herramienta EDA instalada.

---

## Configuración de proyecto (miku.toml)

```toml
[project]
name = "my_chip"
pdk = "sky130A"
layout_source = "magic"   # "magic" | "klayout" | "python"

[tools.xschem]
version_min = "3.1.0"

[tools.klayout]
prefer_python_api = true

[fallback]
sch = ["xschem", "text"]   # si xschem no está, usar diff de texto
gds = ["klayout", "strmcmp", "none"]
```

---

## Notas de diseño

**¿Por qué `parsers/` separado de `adapters/`?**
El parser de `.sch` no necesita Xschem instalado — lee texto plano. El adapter sí necesita el binario. Separarlos permite testear el diff semántico en cualquier entorno, incluyendo CI sin herramientas EDA.

**¿Por qué `pygit2` y no `subprocess` + `git cat-file`?**
`pygit2` usa libgit2 (C) internamente — misma velocidad, pero con API Python limpia, sin parsing de stdout, y sin riesgo de command injection si los paths vienen de input externo.

**¿Por qué `~/.cache/riku/` y no `.riku/cache/` en el repo?**
La caché es por usuario, no por proyecto. El mismo GDS procesado en dos repos distintos comparte la entrada de caché por blob hash. Además, evita contaminar el working tree del repo de chips del usuario.
