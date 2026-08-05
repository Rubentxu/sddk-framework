# HANDOFF — Validación Real SDDK (Gate 1.0.0)

**Última actualización:** 2026-08-05
**Estado:** Validación 3/3 completada · Gate 5/6 · **G1 pendiente bloquea v1.0.0**

---

## 🎯 Estado del Gate 1.0.0

| # | Criterio | Estado | Evidencia |
|---|----------|--------|-----------|
| 1 | ≥3 proyectos validados | ✅ PASS | fd + zoxide + hyperfine |
| 2 | 100% adopt_success | ✅ PASS | 3/3 adoption complete |
| 3 | 0 regresiones | ✅ PASS | 405 tests green (268+21+116) |
| 4 | ≥70% first-pass | ✅ PASS (100%) | 3/3 fixes a la primera |
| 5 | Gaps del framework cerrados | 🔲 **PENDIENTE** | G1 sin cerrar |
| 6 | Report publicado | ✅ PASS | docs/validation/validation-report.md (PR #51) |

**Decisión:** NO publicar v1.0.0 hasta cerrar G1 y re-evaluar.

## 🔴 PRÓXIMO PASO (única tarea pendiente)

**Ciclo SDDK: cerrar gap G1 — `sddk adopt apply` debe plantar `workflow/workflow.yaml`**

- **Problema:** `sddk cycle start` requiere `workflow/workflow.yaml` en el repo del proyecto. `adopt apply` NO lo planta → el script de validación tuvo que copiarlo manualmente.
- **Fix propuesto:** `adopt apply` planta el manifest canónico `workflow/workflow.yaml` (o `cycle start` usa fallback al embebido en el binario).
- **Archivos:** `crates/sddk-cli/src/adopt*.rs`, `crates/sddk-cli/src/cycle.rs`, workflow embebido (const WORKFLOW_MANIFEST en lib.rs).
- **Verificación:** re-run `./scripts/validate-project.sh` sin el cp manual del workflow → ciclo debe abrir.
- **Tras cerrar:** re-evaluar gate → si 6/6 → v1.0.0.

## 📦 Entregables de la Validación

### Infraestructura (PR #50 — MERGED, commit 059b8f4)
| Archivo | Propósito |
|---------|-----------|
| `scripts/validate-project.sh` | Pipeline automatizado: container → clone → adopt → cycle → baseline tests → implement → report.json |
| `scripts/sddk-validate.container` | Quadlet systemd (rust:1.91-slim, CPUQuota=800%, MemoryMax=16G, Network=none) |

**Uso:** `./scripts/validate-project.sh <owner/repo> <issue>`
**Output:** `~/.sddk-validate/{project}/{report.json, logs/, clone/}`

### Reporte (PR #51 — OPEN)
- `docs/validation/validation-report.md` — tabla comparativa + gaps + recomendación

## 📊 Resultados de los 3 Proyectos

| Proyecto | Issue | Tipo | Fix | LOC | Tests pre→post | Verificación |
|----------|-------|------|-----|-----|----------------|--------------|
| sharkdp/fd | #2081 | Bug panic | `UNIX_EPOCH.checked_add()` en `src/filter/time.rs:45` + test regresión | +15 | 268→268 | `@u64::MAX` → error graceful, no panic |
| ajeetdsouza/zoxide | #1273 | Feature | comando `export` (plain/json/csv) — `src/cmd/export.rs` nuevo | +137 | 16→21 | 3 formatos verificados, completions auto-regen |
| sharkdp/hyperfine | #915 | Bug seguridad | `sanitize_csv_cell` en `src/export/csv.rs` (prefijo `'` para `=+-@\t\r`) | +272 | 39→116 | 15 tests csv green, safe values intactos |

**Sandbox:** `~/.sddk-validate/` — fd/, zoxide/, hyperfine/, sddk-bin/, cargo-target/ (persistente), cargo-cache/

## 🐛 Gaps Detectados (todos en el script EXCEPTO G1)

| Gap | Severidad | Estado |
|-----|-----------|--------|
| G1: cycle start requiere workflow.yaml plantado | MEDIUM | **PENDIENTE — fix de framework** |
| G2: API `adopt apply` confusa | LOW | Opcional (alias) |
| G3: containers pierden cargo target | MEDIUM | ✅ corregido (volumen persistente) |
| G4: layout logs inconsistente | LOW | Opcional (doc) |

## 🛠️ Know-How Técnico del Sandbox

- **Quadlet:** keys válidas en [Container] son `Image/ContainerName/Volume/Network/Exec`; límites van en [Service] (`CPUQuota=800%`, `MemoryMax=16G`). NO usar `Cpus`/`WorkingDirectory` (error de generación).
- **SELinux Enforcing:** todos los mounts necesitan `:Z` (relabel) o `:ro,Z`.
- **Containers efímeros:** `podman run` pierde `/tmp` entre invocaciones → CARGO_TARGET_DIR debe montarse en volumen persistente (`~/.sddk-validate/cargo-target`).
- **Binarios:** con CARGO_TARGET_DIR montado, el binario está en `/target/debug/`, NO en `./target/debug/` del repo.
- **Imagen:** `docker.io/library/rust:1.91-slim` — rustc 1.91.1, coincide con rust-toolchain.toml.
- **zoxide tests:** unit en binario (16), integration tests dir (0 en container); `cargo test` sin `--lib`.
- **hyperfine tests:** unit 58 + execution_order 19 + integration 39.
- **fd tests:** 157 unit + 111 integration = 268.

## 📁 Ciclo Ledger

- `p-d9539a6a4bea1de0/validation-sandbox` — **CLOSED** (fase archive)
- Ciclos de validación en sandbox: `p-45ef15bfb213899a/validation-fd`, `p-447a3d6d0da5a761/validation-zoxide`, `p-8231512be3fcc572/validation-hyperfine` — todos OPEN (explore) en sus sandboxes (state dentro de containers, no persiste entre sesiones — OK, fueron evidencia)

## 📝 Convenciones de la Sesión

- Usuario habla español; sesión en español
- Trunk-based: feature branch → PR → auto-merge (CI "Required quality gates" + conversation resolution, reviews=0)
- Auto-merge: `.github/workflows/auto-merge.yml` — sin filtro de rama (PR corre en feature branch)
- Los fixes a proyectos externos NO requieren merge del maintainer — el PR/branch + tests green es evidencia suficiente
