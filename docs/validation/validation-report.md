# SDDK Validation Report — Real-Project Gate 1.0.0

**Date:** 2026-08-05
**Sandbox:** podman quadlet (rust:1.91-slim, 8 cpu/16G bound, SELinux Enforcing)
**Scope:** 3 proyectos reales de GitHub, 3 issues reales resueltos

---

## Resumen Ejecutivo

**SDDK ha sido validado end-to-end en 3 proyectos reales de GitHub.** En cada uno: adopción completada, ciclo A-lite abierto, issue real explorado con root cause verificado, fix/feature implementado, tests del proyecto en verde, verificación manual. **3/3 first-pass, 0 regresiones, 405 tests green en total.**

## Tabla Comparativa

| Proyecto | Issue | Tipo | Fix | LOC | Tests antes | Tests después | Verificación |
|----------|-------|------|-----|-----|-------------|---------------|--------------|
| sharkdp/fd | #2081 | Bug (panic) | `checked_add` 1 línea + test | 15 | 268 | 268 + 1 regresión ✅ | `@u64::MAX` → error graceful, no panic |
| ajeetdsouza/zoxide | #1273 | Feature | comando `export` (3 formatos) | 137 | 16 | 21 (5 nuevos) ✅ | plain/json/csv verificados manualmente |
| sharkdp/hyperfine | #915 | Bug (seguridad) | sanitize CSV cells | 272 | 39 | 116 (77 nuevos) ✅ | 15 tests csv green, safe values intactos |

**Totales:** 405 tests green (0 regresiones), 3/3 first-pass, 424 LOC de cambio en proyectos externos.

## Gate 1.0.0 — Evaluación

| # | Criterio | Resultado | Evidencia |
|---|----------|-----------|-----------|
| 1 | ≥3 proyectos validados | ✅ **PASS** | fd + zoxide + hyperfine |
| 2 | 100% adopt_success | ✅ **PASS** | 3/3 adoption.json + ledger |
| 3 | 0 regresiones | ✅ **PASS** | 405 tests green post-fix |
| 4 | ≥70% first_pass | ✅ **PASS** (100%) | 3/3 fixes a la primera |
| 5 | Gaps del framework cerrados | 🔲 **PENDIENTE** | 4 gaps → ciclos de gap |
| 6 | Report publicado | ✅ **PASS** (este doc) | — |

**Veredicto: 5/6 criterios PASS.** Falta cerrar los 4 gaps del framework (criterio 5).

## Gaps del Framework Detectados (de integración real)

| # | Gap | Severidad | Fix propuesto |
|---|-----|-----------|---------------|
| G1 | `cycle start` requiere `workflow/workflow.yaml` plantado manualmente en el repo | MEDIUM | `sddk adopt apply` debería plantar el manifest canónico (o `sddk cycle start` fallback al embebido) |
| G2 | API `adopt apply --root` confusa (no `adopt --root`) | LOW | Alias / help más claro |
| G3 | Containers efímeros pierden cargo target entre runs | MEDIUM | Volumen persistente (ya corregido en script: `cargo-target` volume) |
| G4 | Layout de outputs inconsistente (logs en clone/logs vs logs/) | LOW | Documentar en README del script |

**Nota G1 es el gap real de framework** — los demás son del script de validación (ya mitigados).

## Recomendación

- **NO publicar v1.0.0 todavía** (criterio 5 pendiente)
- Prioridad: ciclo de gap para **G1** (workflow manifest en adopt) — es el único que requiere cambio de framework
- G2/G4: mejoras UX menores, pueden ir con G1 o en ciclo separado
- Tras cerrar gaps → re-evaluar gate → v1.0.0

## Automatización (reutilizable)

```
./scripts/validate-project.sh <owner/repo> <issue>
# → container efímero → clone → adopt → cycle → baseline tests → implement → tests → report.json
# Outputs: ~/.sddk-validate/{project}/{report.json, logs/, clone/}
```

Pipeline probado 3 veces, idempotente (re-clona si falta, usa cargo-target persistente).
