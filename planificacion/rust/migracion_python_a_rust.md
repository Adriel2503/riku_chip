# Migración Python → Rust: decisiones y estado

## Decisión de migrar (2026)

Se decidió migrar de Python a Rust como implementación principal. Las razones:

1. **Compañero avanzando con gdstk/rust**: la integración futura con el layout GDS es en Rust
2. **Un solo lenguaje**: mantener Python + Rust en paralelo es deuda doble en cada cambio de algoritmo
3. **Distribución**: un binario estático es más fácil de instalar en entornos de EDA (Docker, CI)

**Lo que NO motivó la migración**: rendimiento. Los benchmarks mostraron que Python no era el cuello de botella (parser 7µs/comp, git 1.39ms/blob). La migración es estratégica, no de performance.

## Estado de paridad por módulo

| Módulo Python | Módulo Rust | Estado |
|---------------|-------------|--------|
| `riku/parsers/xschem.py` | `riku_rust/src/parsers/xschem.rs` | ✅ paridad completa |
| `riku/core/semantic_diff.py` | `riku_rust/src/core/semantic_diff.rs` | ✅ paridad completa |
| `riku/core/git_service.py` | `riku_rust/src/core/git_service.rs` | ✅ paridad completa |
| `riku/core/analyzer.py` | `riku_rust/src/core/analyzer.rs` | ✅ paridad completa |
| `riku/core/svg_annotator.py` | `riku_rust/src/core/svg_annotator.rs` | ✅ paridad completa |
| `riku/adapters/xschem_driver.py` | `riku_rust/src/adapters/xschem_driver.rs` | ✅ paridad completa |
| `riku/cli.py` (Typer) | `riku_rust/src/cli.rs` (Clap) | ✅ paridad completa |

## Diferencias intencionales entre Python y Rust

| Aspecto | Python | Rust | Razón |
|---------|--------|------|-------|
| Tipos de colección | `dict` | `BTreeMap` | Orden determinista en tests |
| Error handling | Excepciones | `Result<T, E>` con thiserror | Idiomático en Rust |
| Caché de versión | Variable de clase | `OnceLock<DriverInfo>` | Thread-safe sin Mutex |
| Regex | `re.compile()` módulo | `Lazy<Regex>` static | Una sola compilación |
| CLI | Typer + Click | Clap derive | Ecosistema Rust |

## Calibración mooz: algoritmo igual en ambos

El algoritmo de calibración de coordenadas SVG es idéntico en Python y Rust:

1. Si existe `origins.txt`: usar `xorigin`, `yorigin` + calibrar `mooz` desde wire endpoints
2. Threshold de matching: distancia < 8px
3. Outlier rejection: descartar muestras a >2σ de la media
4. Fallback: mínimos cuadrados libre con eliminación de outliers

## Qué queda del lado Python

Los scripts de test/benchmark en `tests/` del repo raíz son Python:
- `test_xschem_parser.py`, `test_git_service.py`, `test_analyzer.py`, `test_svg_annotator.py`
- Benchmarks: `benchmark_speed.py`, `bench_*.py`

**Plan**: reemplazar con tests Rust en `riku_rust/tests/`. El stress test ya está en `tests/stress.rs`. Los benchmarks se pueden portar como criterion benchmarks si hace falta.

## Integración con gdstk/rust (fase 9)

Cuando el compañero tenga el módulo gdstk/rust listo, la integración puede ser:

**Opción A: workspace Cargo**
```toml
# riku_chip/Cargo.toml
[workspace]
members = ["riku_rust", "gdstk/rust"]
```
Permite compartir tipos entre crates sin publicar a crates.io.

**Opción B: path dependency**
```toml
# riku_rust/Cargo.toml
[dependencies]
gdstk-rust = { path = "../gdstk/rust" }
```
Más simple, no requiere workspace. Recomendado para empezar.

**Decisión pendiente**: coordinar con el compañero qué API expone gdstk/rust (trait, structs, función libre).

## Plan de retiro de Python

1. ~~Completar paridad funcional~~ ✅
2. Migrar tests manuales a `tests/stress.rs` y agregar benchmarks con criterion ← en progreso
3. Confirmar que `riku_rust` pasa stress test en Docker con xschem real
4. Eliminar `riku/` del repo (o moverlo a `archivo/`)
5. Actualizar `README.md` para apuntar solo al binario Rust

No hay urgencia en el retiro formal. El criterio es: si nadie toca `riku/` en 30 días, se archiva.
