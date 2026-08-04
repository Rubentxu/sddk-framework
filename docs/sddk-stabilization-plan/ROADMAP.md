# Roadmap de entrega — SDDK v3.6

**Estado auditado:** 2026-08-04
**Baseline:** `v0.2.0`
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> Los números PR 1-9 representan unidades funcionales del plan, no números literales de pull request. `v0.2.0` cierra canon, inventario y CI de PR1-PR3 sobre la base Rust entregada en `v0.1.0`.

## Panel de entrega

| PR | Estado actual | Gate | Bloqueo principal |
| --- | --- | --- | --- |
| PR 1 | Completo | CI + SDDK001-SDDK010 | Contrato único e inventario generado demostrados. |
| PR 2 | Completo | Required quality gates | Workspace, linter, generadores y testkit tienen pruebas y CI. |
| PR 3 | Completo | Tests Rust + adopción | UUID persistido, XDG y reparación están alineados con el workflow. |
| PR 4 | Completo | Tests Rust + CLI end-to-end | Ciclo, fases, ledger, leases/fencing y rebuild expuestos por CLI y probados. |
| PR 5 | Completo | Gateway + Git + CAS probados | Capability gateway default-deny, runner tipado, filesystem scoped, Git local con postcondiciones y CAS SHA-256. |
| PR 6 | Parcial | No demostrado | Solo schema/modelo; faltan adapter legacy y permisos por fase. |
| PR 7 | Parcial en modo legacy | No demostrado | Release corregido en prompts, pero no existe Forge ni reconciliación Rust. |
| PR 8 | No iniciado | No demostrado | Sin parser de vault, FTS5, backlinks ni `petgraph`. |
| PR 9 | No iniciado | No demostrado | Sin `xtask`, CI/CD, receipts, SBOM ni attestations. |

## Próximo corte recomendado

1. **No habilitar efectos externos:** capability gateway y approvals default-deny deben preceder a Git/Forge.
2. **Continuar por dependencia:** PR 5 → PR 6 → PR 7 → PR 8 → PR 9.

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

**Estado actual:** Parcial; schema/modelo presentes, integración y permisos ausentes.

### Entregables

- Schemas completos.
- Adaptador legacy.
- Permisos por fase.
- Registro de procedencia.

### Gate

Un agente no puede cambiar de fase mediante texto libre.

## PR 7 — Forge y release

**Estado actual:** Parcial en prompts/shell; runtime Forge no iniciado.

### Entregables

- Trait `Forge`.
- Adaptador GitHub.
- Release plan/apply/reconcile.

### Gate

Un fallo durante merge o publicación se reconcilia sin duplicar efectos.

## PR 8 — Vault, índices e Inspector mínimo

**Estado actual:** No iniciado.

### Entregables

- Parser Markdown/frontmatter.
- Backlinks.
- FTS5.
- Grafo `petgraph`.
- HTML autocontenido.

### Gate

El índice puede borrarse y reconstruirse desde el vault.

## PR 9 — Distribución

**Estado actual:** No iniciado.

### Entregables

- `cargo xtask install-dev`.
- Release-plz.
- Dist.
- Checksums.
- SBOM.
- Attestations.
- Instalación side-by-side.

### Gate

Una versión puede instalarse, verificarse, promoverse y revertirse de forma atómica.

## Orden recomendado

No ejecutar PR 7 antes de que PR 4, PR 5 y PR 6 estén consolidados. No introducir LadybugDB dentro de v3.6.

La consolidación exige: cambios versionados, CI obligatoria, criterios del backlog demostrados y ausencia de gaps P0 abiertos en [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md).
