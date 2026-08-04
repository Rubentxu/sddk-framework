# Roadmap de entrega — SDDK v3.6

**Estado auditado:** 2026-08-04
**Baseline:** `main` en `0fcb09c` más cambios sin commit en el worktree
**Informe:** [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md)

> Los números PR 1-9 representan unidades de entrega planificadas, no pull requests ya integradas. El código Rust, schemas y workflow actuales siguen sin formar parte del historial Git de `main`.

## Panel de entrega

| PR | Estado actual | Gate | Bloqueo principal |
| --- | --- | --- | --- |
| PR 1 | Parcial, sin entregar | No demostrado | Falta inventario generado, contrato único y commit/PR verificable. |
| PR 2 | Parcial, sin entregar | No demostrado | No existe CI; `sddk-testkit` carece de fixtures; schema validation es parcial. |
| PR 3 | Implementado con desviación | Funcional en tests | UUID fallback contradice `workflow.project_identity.fallback: hostname-path`. |
| PR 4 | Parcial | API interna verde | CLI de ledger/ciclos ausente; frames y leases no están integrados extremo a extremo. |
| PR 5 | No iniciado; primitives parciales | No demostrado | No existe capability gateway, runner, Git local ni CAS. |
| PR 6 | Parcial | No demostrado | Solo schema/modelo; faltan adapter legacy y permisos por fase. |
| PR 7 | Parcial en modo legacy | No demostrado | Release corregido en prompts, pero no existe Forge ni reconciliación Rust. |
| PR 8 | No iniciado | No demostrado | Sin parser de vault, FTS5, backlinks ni `petgraph`. |
| PR 9 | No iniciado | No demostrado | Sin `xtask`, CI/CD, receipts, SBOM ni attestations. |

## Próximo corte recomendado

1. **Base versionada:** excluir `target/`, resolver el drift de contratos y convertir PR1-PR4 en work units revisables.
2. **Gate automático:** CI mínima con fmt, test, Clippy estricto, linter, docs check y tests contractuales.
3. **Cerrar autoridad local:** exponer ciclo/fase/ledger por CLI e integrar leases, frames y replay.
4. **No habilitar efectos externos:** capability gateway y approvals default-deny deben preceder a Git/Forge.
5. **Continuar por dependencia:** PR 5 → PR 6 → PR 7 → PR 8 → PR 9.

## PR 1 — Estabilización semántica

**Estado actual:** Parcial, no entregado.

### Alcance

- Corregir adopción, release y ramas.
- Unificar paths.
- Resolver debt verification.
- Eliminar referencias rotas.
- Añadir inventario generado.

### Gate

No existen dos definiciones incompatibles de una misma regla operativa.

## PR 2 — Workspace Rust y linter

**Estado actual:** Parcial, no entregado.

### Entregables

- `sddk-domain`.
- `sddk-engine` mínimo.
- `sddk-storage` mínimo.
- `sddk-cli`.
- `sddk-testkit`.
- `sddk lint`.
- `sddk generate docs`.

### Gate

CI detecta referencias rotas, placeholders y documentación generada desactualizada.

## PR 3 — Identidad, paths y adopción

**Estado actual:** Implementado y probado, con desviación contractual abierta.

### Entregables

- Resolución de proyecto y workspace.
- Paths XDG.
- Registro de adopción atómico.
- Comandos `adopt plan/apply/status/repair`.

### Gate

Dos repositorios con igual nombre no colisionan y una adopción interrumpida es reparable.

## PR 4 — Ledger y máquina de estados

**Estado actual:** Parcial; primitives implementados, superficie operativa incompleta.

### Entregables

- SQLite.
- Migraciones.
- Frames y cadena hash.
- Ciclos y fases.
- Replay.

### Gate

Replay reconstruye el mismo estado lógico y las transiciones inválidas se rechazan.

## PR 5 — Gateway de capacidades locales

**Estado actual:** No iniciado como gateway; receipts y metadata son solo foundations.

### Entregables

- Filesystem tipado.
- Process runner.
- Git local.
- Testing.
- Artefactos por hash.

### Gate

Toda acción local relevante queda registrada y es idempotente.

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
