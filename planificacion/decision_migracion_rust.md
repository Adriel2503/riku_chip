# Plan de Migración a Rust

Cuándo, qué y cómo migrar módulos de Python a Rust en Riku.
Principio rector: **medir primero, migrar después**. Rust no entra por preferencia sino por evidencia.

---

## Regla general

Un módulo se migra a Rust cuando se cumplen **las tres condiciones** simultáneamente:

1. **Hay un benchmark que demuestra el problema** — no una estimación, un número real medido en hardware representativo.
2. **La caché no resuelve el problema** — si el resultado se puede cachear por hash de commit, el costo se paga una sola vez y Rust no aporta nada.
3. **El cuello de botella está en Python** — no en I/O de disco, no en el fork de proceso de una herramienta externa, no en la red. Si el problema es que Xschem tarda 3 s en generar un SVG, Rust no ayuda.

---

## Casos concretos ordenados por probabilidad

### Caso 1 — Parser GDS streaming
**Módulo:** extracción y parsing de archivos `.gds` / `.oas`
**Umbral de migración:** diff de GDS > 200 MB tarda > 30 s con `klayout.db` en Python, medido en hardware de workstation real (8-core, 32 GB RAM).

**Por qué ocurre:** `klayout.db` carga el GDS completo en memoria para operar. Un GDS de 2 GB puede requerir 20–40 GB de RAM al aplanar la jerarquía. Sin tiling, colapsa.

**Solución en Rust:** parser SAX-style que lee el GDS en chunks sin cargar todo en memoria. La librería `gds21` (UC Berkeley) es el punto de partida — actualmente carga en memoria también, pero la arquitectura es extensible a streaming.

**Interfaz:** PyO3. La función Python `parse_gds(path) → List[Cell]` se reemplaza por la extensión Rust sin cambiar el código que la llama.

**Condición de refutación:** si `klayout.db` con tiling (`xor_tile_size = "1.mm"`, `xor_threads = N`) resuelve el problema de memoria, no se necesita Rust para este caso.

---

### Caso 2 — Parser de archivos `.raw` de NGSpice
**Módulo:** lectura de resultados de simulación (waveforms)
**Umbral de migración:** `spicelib` tarda > 10 s en cargar un `.raw` de > 500 MB, medido en uso interactivo real.

**Por qué ocurre:** `spicelib` carga el archivo `.raw` completo en memoria. Simulaciones Monte Carlo de 100 runs o transient de circuitos grandes producen archivos de varios GB.

**Solución en Rust:** parser streaming que extrae solo las señales solicitadas sin cargar el archivo completo. Permite consultas tipo "dame solo la señal `VOUT` entre t=1ns y t=10ns".

**Interfaz:** PyO3. `parse_raw(path, signals=["VOUT", "VDD"]) → Dict[str, Array]`.

**Condición de refutación:** si el uso real de Riku nunca requiere cargar `.raw` de > 500 MB en modo interactivo (los resultados de `.meas` son suficientes para el diff de simulación), este caso no se activa.

---

### Caso 3 — Motor de diff GDS geométrico
**Módulo:** comparación XOR de layouts
**Umbral de migración:** el XOR de un GDS de 500 MB tarda > 60 s después de aplicar caché, medido en CI (runner de 4 cores).

**Por qué ocurre:** el XOR geométrico requiere aplanar la jerarquía completa y operar sobre todos los polígonos. KLayout con `klayout -b -r xor.drc` usa tiling + threads pero sigue siendo lento para chips completos.

**Solución en Rust:** motor de XOR tileado que procesa regiones en paralelo con Rayon. Más control sobre paralelismo que KLayout batch.

**Interfaz:** PyO3 o CLI separado invocado como subprocess.

**Condición de refutación:** si KLayout batch con tiling y threads resuelve el problema dentro del umbral, no se migra. KLayout ya está escrito en C++ — difícil superarlo en velocidad para operaciones que ya soporta.

---

### Caso 4 — Operaciones Git de alto volumen
**Módulo:** `GitService` — extracción de blobs, listado de commits
**Umbral de migración:** listar el historial de un repo con > 10,000 commits tarda > 2 s, medido en uso interactivo.

**Por qué ocurre:** `pygit2` usa libgit2 (C) — en la práctica este caso es muy improbable. `pygit2` ya es rápido.

**Solución en Rust:** reemplazar `pygit2` por `gitoxide` (Rust puro, ~25% más rápido que libgit2 según benchmarks). Pero `gitoxide` aún no tiene `push` completo ni `merge`.

**Interfaz:** reescribir `git_service.py` completo como extensión PyO3.

**Condición de refutación:** muy probable. `pygit2` con C backend raramente es el cuello de botella. Este caso casi seguramente nunca se activa.

---

### Caso 5 — Parser `.sch` de Xschem
**Módulo:** `parsers/xschem.py`
**Umbral de migración:** parsear un `.sch` tarda > 100 ms, medido en esquemáticos reales grandes.

**Por qué ocurre:** los archivos `.sch` son texto plano < 1 MB típicamente. Este caso es prácticamente imposible.

**Condición de refutación:** casi certeza. Los `.sch` son demasiado pequeños para que Python sea un cuello de botella aquí. **Este caso no se migrará.**

---

## Resumen de prioridades

| Caso | Probabilidad de migrar | Impacto si se migra | Complejidad |
|---|---|---|---|
| Parser GDS streaming | Alta | Crítico — desbloquea chips reales | Media |
| Parser `.raw` NGSpice | Media | Alto — simulaciones grandes | Media |
| Motor XOR geométrico | Media | Alto — CI lento | Alta |
| GitService | Baja | Bajo — ya es C por debajo | Alta |
| Parser `.sch` | Ninguna | Ninguno | — |

---

## Cómo se hace la migración cuando llega

1. **Medir primero:** añadir timing en el módulo Python con `time.perf_counter()`. Registrar en condiciones reales.
2. **Verificar que caché no resuelve:** si el resultado es cacheable por hash de commit, agregar caché antes de migrar.
3. **Implementar en Rust con PyO3:** el módulo Rust expone exactamente la misma firma de función que el Python actual.
4. **Tests de paridad:** los tests existentes del módulo Python corren sin cambios contra la extensión Rust. Si pasan, la migración es correcta.
5. **Benchmark post-migración:** medir que el umbral se superó. Si no, revertir.

---

## Lo que Rust nunca reemplaza en Riku

- El CLI (`cli.py`) — es orquestación, no cálculo.
- Los adapters (`xschem_adapter.py`, `klayout_adapter.py`) — el cuello de botella es la herramienta externa, no el código Python que la invoca.
- La UI Qt — Qt ya está en C++, PyQt es un binding delgado.
- La caché SQLite — SQLite ya es C, `sqlite3` de Python es un binding.
