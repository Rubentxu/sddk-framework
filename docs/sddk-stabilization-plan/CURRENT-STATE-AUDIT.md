# Auditoría de estado actual — SDDK v3.6

**Fecha:** 2026-08-04
**Baseline Git:** `v0.1.0` (`ee7957f`) más el corte `feat/canonical-ci-gates`
**Veredicto:** baseline versionada y protegida por gates; v3.6 aún no es estable porque faltan fronteras runtime autoritativas

## Respuesta ejecutiva

La implementación ha avanzado por delante del roadmap escrito: identidad, adopción, SQLite, ledger, máquina de estados, linter y generación de documentación ya tienen código y pruebas. Sin embargo, esos componentes todavía no forman el runtime autoritativo prometido por el PRD.

El estado aceptado del backlog es:

| Estado | Historias | Porcentaje |
| --- | ---: | ---: |
| Completa | 9 | 28 % |
| Parcial | 9 | 28 % |
| Desviada | 0 | 0 % |
| No iniciada | 14 | 44 % |

Los principales bloqueos no son volumen de código. Son fronteras de autoridad:

1. El CLI no expone ciclos, fases, ledger, capabilities, reconcile, vault ni release.
2. Los gates son afirmaciones del caller, no evaluaciones autorizadas con receipt.
3. No existe capability gateway; Git, Forge, filesystem y approvals siguen sin frontera runtime.

## Alcance y método

Se contrastaron:

- `PRD.md`, `ROADMAP.md`, `BACKLOG.md`, `MIGRATION.md` y ADR-0001 a ADR-0008.
- `workflow/workflow.yaml` y `schemas/*.json` raíz.
- La procedencia del paquete bajo `docs/sddk-stabilization-plan/`; sus snapshots ejecutables duplicados ya fueron retirados.
- Los cinco crates Rust y sus tests.
- Los contratos legacy de agentes, skills, prompts y shell.
- El estado Git y los gates ejecutables disponibles.

La clasificación exige evidencia en repositorio. Un tipo, tabla o campo aislado no cierra una historia si el criterio requiere integración operativa.

## Hallazgos priorizados

| ID | Severidad | Estado | Hallazgo | Acción inmediata |
| --- | --- | --- | --- | --- |
| GAP-001 | P0 | Cerrado | Runtime y documentación fundacional versionados en `v0.1.0`; outputs de build excluidos. | Mantener commits por work unit. |
| GAP-002 | P0 | Cerrado | `.github/workflows/ci.yml` ejecuta gates Rust, linter, generados y contratos. | Mantener `Required quality gates` como check obligatorio. |
| GAP-003 | P0 | Abierto | El CLI solo expone project/adopt/lint/generate. | Añadir cycle/phase/ledger y conectar Engine + leases + storage. |
| GAP-004 | P0 | Abierto | `TransitionEvidence.gates` acepta Passed/Failed del caller. | Introducir GateEvaluator, GateReceipt y autorización del emisor. |
| GAP-005 | P0 | Abierto | Capability gateway, runner y policy engine no existen. | Mantener efectos externos deshabilitados hasta implementar default-deny. |
| GAP-006 | P0 | Cerrado | Root workflow/schemas son la única autoridad ejecutable; se retiraron snapshots divergentes. | Impedir nuevas copias mediante revisión y linter. |
| GAP-007 | P1 | Cerrado | Workflow, código y tests usan fallback UUID persistido. | Mantener el receipt como semilla estable. |
| GAP-008 | P1 | Abierto | Ledger hash existe, pero no hay CLI verify ni reconstrucción en base vacía. | Cerrar SDDK-402/404 con comandos y fixture de rebuild. |
| GAP-009 | P1 | Abierto | Frames y leases son primitives sin enforcement extremo a extremo. | Integrarlos en toda mutación y rechazar fencing tokens obsoletos. |
| GAP-010 | P1 | Abierto | Artifact metadata no es un CAS; SHA-256 es opcional y no se calcula. | Implementar store por contenido y digest obligatorio. |
| GAP-011 | P1 | Abierto | Receipts permiten insertar directamente estados terminales y JSON sin sanear. | Separar begin/finalize/reconcile y aplicar schemas/redacción. |
| GAP-012 | P1 | Abierto | Forge/release está corregido solo en prompts y shell. | Implementar puerto Forge, adaptador GitHub y release reconciliable. |
| GAP-013 | P1 | Cerrado | `sddk-testkit::TestRepository` ofrece fixture reutilizable con aislamiento de paths. | Extenderlo cuando storage y capabilities requieran harness compartido. |
| GAP-014 | P2 | Abierto | Vault, FTS5, backlinks y petgraph no están iniciados. | Mantener PR8 detrás de PR5-PR7. |
| GAP-015 | P2 | Abierto | Distribución, receipts, SBOM y attestations no están iniciados. | Mantener PR9 como último corte. |

## Detalle de bloqueos P0

### GAP-001 — Base entregada y versionada

La base fundacional se integró en `main` mediante PR #2 y se publicó como `v0.1.0`. `target/` permanece excluido.

El corte actual mantiene las nuevas unidades en una rama trazada por issue y se verificará con el mismo flujo trunk-based.

**Criterio de cierre:** commits separados por comportamiento, revisión de archivos incluidos, `target/` excluido, CI verde y trazabilidad entre cada commit y las historias cerradas.

### GAP-002 — Gates de CI implementados

`.github/workflows/ci.yml` define un único check obligatorio y reproducible sobre Rust 1.85.0.

La CI mínima debe ejecutar:

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo run -q -p sddk-cli -- lint --root . --format json
cargo run -q -p sddk-cli -- generate docs --root . --check
cargo run -q -p sddk-cli -- generate inventory --root . --check
bash tests/test_workflow_contract.sh
bash tests/test_adoption_contract.sh
```

### GAP-003 — Rust no es todavía la autoridad operativa

`crates/sddk-cli/src/lib.rs` expone:

- `project resolve`;
- `adopt plan|apply|status|repair`;
- `lint`;
- `generate docs|inventory`.

No expone los casos de uso centrales del PRD: `cycle start`, `phase complete`, `ledger verify`, `capability plan/apply`, `reconcile`, vault o release. El engine tiene APIs útiles, pero los prompts todavía poseen la ejecución real.

**Criterio de cierre:** un test end-to-end debe recorrer CLI → Engine → Storage para crear ciclo, completar fase, bloquear/desbloquear, verificar ledger y replay.

### GAP-004 — Los gates no prueban nada por sí mismos

`TransitionEvidence` recibe un mapa de `GateOutcome` suministrado por el caller. El engine comprueba presencia y estado, pero no quién evaluó el gate, qué comando se ejecutó, qué política se usó ni si la evidencia corresponde al plan.

Un gate ejecutable necesita:

- evaluador registrado;
- inputs normalizados;
- actor autorizado;
- hash del plan;
- policy version;
- resultado y evidencia sanitizada;
- receipt vinculado a `command_id` y `frame_id`.

`tests-pass`, `policy-compliant`, `review-approved`, `no-pending-effects` y `ledger-valid` no deben aceptar autoafirmación del mismo caller que solicita la transición.

### GAP-005 — Falta la frontera de capacidades

ADR-0005 exige `validate → plan → authorize → apply → verify → receipt`. El workspace solo contiene tipos, metadata y almacenamiento idempotente de receipts.

No existen:

- runner con argv tipado y allowlist de entorno;
- filesystem restringido y seguro frente a symlinks/escapes;
- Git local;
- policy decision point;
- approvals R3/R4;
- lifecycle `started → succeeded|failed|unknown`;
- reconciliación.

Esta ausencia es un bloqueador de diseño antes de habilitar operaciones mutantes o externas. No se clasifica como vulnerabilidad remota activa porque el gateway todavía no está expuesto.

### GAP-006 — Contratos ejecutables unificados

`workflow/workflow.yaml` y `schemas/*.json` son las únicas fuentes ejecutables. Las copias divergentes del paquete se retiraron y el fallback canónico es `receipt-uuid`.

**Regla vigente:** la documentación enlaza los contratos raíz; no se mantienen snapshots ejecutables paralelos.

## Gaps P1 de implementación

### Ledger, replay y concurrencia

- `verify_ledger` detecta huecos y manipulación, pero no tiene comando CLI.
- `replay_cycle` reconstruye el último snapshot desde eventos existentes; no repuebla una base vacía.
- `frame_id` se persiste, pero no se impone la relación command → frame.
- Los leases tienen fencing token, pero ninguna operación del engine exige el token.
- No existe renovación, inspección o recuperación de lease desde CLI.
- Solo existe schema SQLite v1; no hay estrategia probada de backup/migración v2+.

### Artefactos y receipts

- `insert_artifact` almacena metadata; no recibe bytes ni calcula el digest.
- `sha256` es opcional y no existe unicidad/deduplicación por contenido.
- `record_capability_receipt` permite insertar `succeeded` directamente.
- No existe API para finalizar o reconciliar un receipt iniciado.
- Requests, results, payloads y snapshots aceptan JSON arbitrario sin redacción; podrían persistir secretos o PII.
- `actor` y timestamps son caller-supplied, por lo que la atribución es declarativa.

### Forge y release

- `ForgeDef` es configuración, no un trait.
- No existe adaptador GitHub runtime.
- `CapabilityStatus::Unknown` no tiene reconciliador.
- Las 117 pruebas shell protegen el contrato textual de release, no ejecutan una release Rust.
- No hay rollback side-by-side ni receipt de promoción.

### Calidad de la base

- `sddk-testkit` ofrece un fixture repository reutilizable; storage/capabilities aún necesitarán harness especializados.
- README enlaza `docs/generated/inventory.md`; SDDK010 detecta drift sobre los 64 agentes y 90 skills actuales.
- Los schemas existen, pero ningún JSON Schema validator se ejecuta en runtime.

## Estado por PR

| PR | Implementación observada | Estado de aceptación |
| --- | --- | --- |
| PR 1 | Hotfix semántico, contrato único e inventario generado | Completo y protegido por CI. |
| PR 2 | Cinco crates, testkit, linter, generadores y CI | Completo; JSON Schema runtime queda en SDDK-101, no bloquea esta unidad. |
| PR 3 | Identidad UUID, XDG y adopción reparable | Completo y alineado con el workflow. |
| PR 4 | SQLite, hash chain, engine, replay y leases | Parcial; APIs internas sin superficie CLI ni integración de concurrencia. |
| PR 5 | Receipts y artifact metadata como foundations | No iniciado como gateway. |
| PR 6 | AgentResult y schema | Parcial; adapter y permisos no iniciados. |
| PR 7 | Contrato legacy de release con tests | Parcial; Forge/reconcile runtime no iniciados. |
| PR 8 | ADRs y templates | No iniciado. |
| PR 9 | ADR de distribución | No iniciado. |

## Verificación ejecutada

| Gate | Resultado |
| --- | --- |
| `cargo test --workspace --locked` | PASS, 88 tests en el corte. |
| `sddk lint --format json` | PASS, 0 errores y 0 warnings. |
| `sddk generate docs --check` | PASS, documentación actual. |
| `sddk generate inventory --check` | PASS, 64 agentes y 90 skills. |
| `tests/test_workflow_contract.sh` | PASS, 117 checks. |
| `tests/test_adoption_contract.sh` | PASS, 22 checks. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS. |
| CI remota | Definida como `Required quality gates`; pendiente de evidencia de GitHub en el PR del corte. |

## Plan de acción recomendado

### Work unit A — Canon y baseline

**Estado:** completado entre `v0.1.0` y el corte de canon.

**Acciones:**

1. Resolver UUID vs `hostname-path`.
2. Declarar root workflow/schemas como autoridad única.
3. Marcar o retirar snapshots incompatibles del paquete.
4. Corregir `.gitignore` para excluir `target/`.
5. Separar commits PR1, PR2, PR3 y PR4 por comportamiento.

**Gate:** checkout limpio reproduce todos los gates locales.

### Work unit B — CI y testkit

**Estado:** implementado en el corte `feat/canonical-ci-gates`.

**Acciones:** CI mínima, testkit con fixture repository reutilizable, Clippy estricto y checks locked.

**Gate:** required check único o conjunto documentado, obligatorio antes de merge.

### Work unit C — Cierre PR4

**Objetivo:** exponer la autoridad local ya construida.

**Acciones:** CLI cycle/phase/ledger, frame invariant, leases/fencing, replay rebuild y errores estables.

**Gate:** test end-to-end sin red ni reloj real que recorra un ciclo y reconstruya estado.

### Work unit D — Capability gateway

**Objetivo:** crear una frontera no eludible antes de Git/Forge.

**Acciones:** policy default-deny, approvals vinculadas al plan, begin/finalize/reconcile receipt, runner tipado, filesystem seguro y redacción.

**Gate:** pruebas negativas demuestran que R3/R4, shell arbitrario, path escape y gate autoafirmado son rechazados.

### Work unit E — Git, CAS y agentes

**Objetivo:** cerrar PR5 y PR6 sobre el gateway.

**Acciones:** Git local, CAS SHA-256, adapter legacy, schema validation y permisos agent/phase/capability.

**Gate:** toda acción produce receipt, postcondición verificada y evento causal.

### Work unit F — Forge/release

**Objetivo:** mover la secuencia ya estabilizada desde prompts a runtime reconciliable.

**Acciones:** trait Forge, GitHub adapter, unknown reconciliation, release plan/apply/reconcile y rollback.

**Gate:** interrupciones simuladas entre merge/tag/publish no duplican efectos y convergen tras reconcile.

### Work units G-H — Vault y distribución

Ejecutar PR8 y PR9 solo después de cerrar los work units anteriores. LadybugDB permanece fuera de v3.6.

## Decisiones que requieren cierre explícito

| Decisión | Opciones | Recomendación |
| --- | --- | --- |
| Fallback sin remote | Cerrada | UUID persistido: mover el checkout no cambia la identidad. |
| Workflow del paquete | Cerrada | Referencia a raíz; no mantener dos contratos ejecutables. |
| Validación de gates | Caller assertion vs receipt autorizado | Receipt autorizado y vinculado al plan. |
| Vault canónico | Paths XDG del runtime vs vault de conocimiento existente | Separar explícitamente estado operativo XDG de conocimiento canónico; documentar ownership y migración. |
| Migración SQLite | Auto-migrate al abrir vs comando explícito | Backup + lock exclusivo + migración explícita para cambios destructivos. |

## Criterio actualizado de salida v3.6

v3.6 no debe declararse estable hasta que:

- el trabajo esté versionado y CI sea obligatoria;
- Rust controle adopción, ciclo, fase, gates, Git local, ledger y recuperación;
- ningún gate crítico acepte una afirmación no autorizada;
- toda capacidad mutante pase por gateway, policy y receipt;
- replay/reconcile recuperen interrupciones sin editar SQLite manualmente;
- Forge/release converja sin duplicar efectos;
- no existan contratos ejecutables duplicados o contradictorios.
