# Roadmap de entrega — SDDK v3.6

**Estado auditado:** 2026-08-04
**Baseline:** `v0.14.0`
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> Los números PR 1-9 representan unidades funcionales del plan, no números literales de pull request. El plan PR1-PR9 se entregó entre `v0.1.0` y `v0.10.0`; v3.6 se declaró estable en `v0.11.0` y se endureció en `v0.12.0`-`v0.14.0`.

## Panel de entrega

| PR | Estado actual | Gate | Bloqueo principal |
| --- | --- | --- | --- |
| PR 1 | Completo | CI + SDDK001-SDDK010 | Contrato único e inventario generado demostrados. |
| PR 2 | Completo | Required quality gates | Workspace, linter, generadores y testkit tienen pruebas y CI. |
| PR 3 | Completo | Tests Rust + adopción | UUID persistido, XDG y reparación están alineados con el workflow. |
| PR 4 | Completo | Tests Rust + CLI end-to-end | Ciclo, fases, ledger, leases/fencing y rebuild expuestos por CLI y probados. |
| PR 5 | Completo | Gateway + Git + CAS probados | Capability gateway default-deny, runner tipado, filesystem scoped, Git local con postcondiciones y CAS SHA-256. |
| PR 6 | Completo | Tests Rust + CLI | Schema validation runtime, adaptador legacy y permisos por fase con default-deny. |
| PR 7 | Completo | Tests Rust + MockForge | Forge trait, adaptador GitHub, release plan/apply idempotente y reconciliación contra el proveedor. |
| PR 8 | Completo | Tests Rust + CLI | Parser de vault, índice FTS5 reconstruible, validación y grafo petgraph. |
| PR 9 | Completo | Tests Rust + CLI | `dev doctor/check/install/verify/uninstall`, dist con checksums/SBOM/attestations y verificación atómica. |

## Próximo corte recomendado

El roadmap está COMPLETO y cerrado. Post-estabilidad: hardening, integración dogfood, registro de agentes, packs declarativos, índice incremental y envolvente de error (`v0.12.0`-`v0.18.0`). Con RNF-006, todos los requisitos del PRD (RF-001 a RF-015, RNF-001 a RNF-006) quedan cubiertos.

## PR 1 — Estabilización semántica

**Estado actual:** Completo; canon único e inventario protegidos por CI.

### Alcance

- Corregir adopción, release y ramas.
- Unificar paths.
- Resolver debt verification.
- Eliminar referencias rotas.
- Añadir inventario generado.

### Gate

No existen dos definiciones incompatibles de una misma regla operativa.

## PR 2 — Workspace Rust y linter

**Estado actual:** Completo; workspace, testkit, linter y generación protegidos por CI.

### Entregables

- `sddk-domain`.
- `sddk-engine` mínimo.
- `sddk-storage` mínimo.
- `sddk-cli`.
- `sddk-testkit`.
- `sddk lint`.
- `sddk generate docs`.
- `sddk generate inventory`.

### Gate

CI detecta referencias rotas, placeholders y documentación generada desactualizada.

## PR 3 — Identidad, paths y adopción

**Estado actual:** Completo; implementado, probado y alineado con el contrato canónico.

### Entregables

- Resolución de proyecto y workspace.
- Paths XDG.
- Registro de adopción atómico.
- Comandos `adopt plan/apply/status/repair`.

### Gate

Dos repositorios con igual nombre no colisionan y una adopción interrumpida es reparable.

## PR 4 — Ledger y máquina de estados

**Estado actual:** Completo; autoridad local expuesta por CLI y probada extremo a extremo.

### Entregables

- SQLite.
- Migraciones.
- Frames y cadena hash.
- Ciclos y fases.
- Replay.
- CLI `cycle start|status|transition|rebuild`, `cycle lock`, `ledger verify|events`.

### Gate

Replay reconstruye el mismo estado lógico y las transiciones inválidas se rechazan. El CLI recorre un ciclo completo (adopt → start → transition → verify → rebuild) sin red ni reloj real y las mutaciones exigen fencing cuando el ciclo está leaseado.

## PR 5 — Gateway de capacidades locales

**Estado actual:** Completo; gateway, Git local y CAS implementados y probados.

### Entregables

- Filesystem tipado.
- Process runner.
- Git local.
- Testing.
- Artefactos por hash.

### Gate

Toda acción local relevante queda registrada y es idempotente. El gateway aplica policy default-deny, approvals R3/R4 y receipts `started → succeeded|failed` con redacción; las operaciones Git verifican postcondiciones y el CAS exige y re-verifica SHA-256.

## PR 6 — Protocolo de agentes

**Estado actual:** Completo; schema validation, adaptador legacy y permisos por fase probados.

### Entregables

- Schemas completos.
- Adaptador legacy.
- Permisos por fase.
- Registro de procedencia.

### Gate

Un agente no puede cambiar de fase mediante texto libre. La validación JSON Schema runtime rechaza resultados inválidos, el adaptador emite warnings de campos no verificables y `PermissionPolicy` niega por defecto agentes/fases/capacidades no declaradas.

## PR 7 — Forge y release

**Estado actual:** Completo; trait Forge, adaptador GitHub, release plan/apply y reconciliación probados.

### Entregables

- Trait `Forge`.
- Adaptador GitHub.
- Release plan/apply/reconcile.

### Gate

Un fallo durante merge o publicación se reconcilia sin duplicar efectos. `apply_release` re-chequea el proveedor antes de cada paso, omite efectos ya presentes y `reconcile_pending` finaliza receipts interrumpidos consultando la realidad.

## PR 8 — Vault, índices e Inspector mínimo

**Estado actual:** Completo; parser, validación, FTS5 reconstruible, grafo e inspector HTML probados.

### Entregables

- Parser Markdown/frontmatter.
- Backlinks.
- FTS5.
- Grafo `petgraph`.
- HTML autocontenido.

### Gate

El índice puede borrarse y reconstruirse desde el vault (`vault index` re-crea la tabla FTS5 desde los nodos).

## PR 9 — Distribución

**Estado actual:** Completo; doctor, gates, instalación con receipt y dist verificable probados.

### Entregables

- `sddk dev doctor|check|install|verify|uninstall` (equivalente a xtask).
- `sddk release dist|verify`.
- Checksums.
- SBOM.
- Attestations.
- Instalación side-by-side.

### Gate

Una versión puede instalarse, verificarse, promoverse y revertirse de forma atómica (`dev install/verify/uninstall` con receipt SHA-256; `release dist/verify` con checksums, SBOM y attestation).

## Orden recomendado

No ejecutar PR 7 antes de que PR 4, PR 5 y PR 6 estén consolidados. No introducir LadybugDB dentro de v3.6.

La consolidación exige: cambios versionados, CI obligatoria, criterios del backlog demostrados y ausencia de gaps P0 abiertos en [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md).

---

## Milestone E2E-2026-08 — Validación E2E ampliada (post-v1.3.0)

**Estado:** planificado (2026-08-06) — ADR-0001 + e2e-plan.md aprobados
**Objetivo:** probar instalación real, despliegue, multi-lenguaje y render de diagramas.

| Work item | Tipo | Depende de | Estado |
|-----------|------|-----------|--------|
| scripts/e2e-install.sh (N1) | feature | ADR-0001 | planificado |
| scripts/e2e-render.sh (N2) | feature | ADR-0001 | planificado |
| validate-project.sh --lang (matrix 5 lenguajes) | feature | ADR-0001 | planificado |
| scripts/e2e-all.sh (orquestador) | feature | N1+N2+matrix | planificado |
| docs/validation/e2e-report.md + evidencia | docs | e2e-all | planificado |
| Checklist N3 (editor real) | docs | dev link | planificado |

**Criterios de salida:**
- 5/5 lenguajes validados (adopt + cycle + tests baseline)
- Instalador probado en sandbox sin git (variantes a-d)
- Diagramas renderizados y verificados visualmente (SVG + screenshots)
- Report E2E publicado con evidencia embebida

## Modo de operación: LOCAL-FIRST (2026-08-06)

**GitHub Actions DESACTIVADO** (minutos del plan agotados; decisión: no depender de CI remoto).
- `actions/permissions.enabled = false` (API)
- Branch protection de main: sin required status checks (nada bloquea merges)
- CI, auto-merge y release automation: **inactivos** — no son parte del flujo operativo
- El flujo de validación y release es **local**: podman (sandbox), scripts `scripts/*.sh`, binario `sddk` (release dist/verify locales), merges vía `gh pr merge`

**Consecuencias:**
- Los PRs se validan localmente (tests + clippy + fmt) antes de mergear
- Los releases se generan localmente (`sddk release dist`) y se publican con `gh release create`
- El milestone E2E-2026-08 se ejecuta 100% local (podman + mmdc)
