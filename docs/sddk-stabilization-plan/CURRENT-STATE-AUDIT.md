# Auditoría de estado actual — SDDK v3.6

**Fecha:** 2026-08-04
**Baseline Git:** `v0.2.0`
**Veredicto:** baseline versionada y protegida por gates; v3.6 aún no es estable porque faltan fronteras runtime autoritativas

## Respuesta ejecutiva

La implementación ha avanzado por delante del roadmap escrito: identidad, adopción, SQLite, ledger, máquina de estados, linter y generación de documentación ya tienen código y pruebas. Sin embargo, esos componentes todavía no forman el runtime autoritativo prometido por el PRD.

El estado aceptado del backlog es:

| Estado | Historias | Porcentaje |
| --- | ---: | ---: |
| Completa | 20 | 63 % |
| Parcial | 0 | 0 % |
| Desviada | 0 | 0 % |
| No iniciada | 12 | 37 % |

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
| GAP-003 | P0 | Cerrado | CLI expone cycle/lock/ledger conectados a Engine + storage. | Añadir capabilities y vault como próximos cortes. |
| GAP-004 | P0 | Abierto | `TransitionEvidence.gates` acepta Passed/Failed del caller. | Introducir GateEvaluator, GateReceipt y autorización del emisor. |
| GAP-005 | P0 | Cerrado | Gateway default-deny con runner tipado, filesystem scoped, approvals R3/R4 y receipts con redacción. | Extenderlo a Git local y CAS antes de habilitar efectos externos. |
| GAP-006 | P0 | Cerrado | Root workflow/schemas son la única autoridad ejecutable; se retiraron snapshots divergentes. | Impedir nuevas copias mediante revisión y linter. |
| GAP-007 | P1 | Cerrado | Workflow, código y tests usan fallback UUID persistido. | Mantener el receipt como semilla estable. |
| GAP-008 | P1 | Cerrado | `sddk ledger verify` y `sddk cycle rebuild` restauran y verifican la base. | Mantener rebuild como primitiva de reparación sin overwrite de divergencias. |
| GAP-009 | P1 | Cerrado | Frames por comando consultables y leases con fencing exigido en mutaciones de ciclos leaseados. | Aplicar el mismo fence a capabilities y Git cuando existan. |
| GAP-010 | P1 | Cerrado | CAS con SHA-256 obligatorio, deduplicación por contenido y verificación en cada lectura. | Usar digest como clave de artefacto en adaptadores futuros. |
| GAP-011 | P1 | Cerrado | Receipts con lifecycle begin→finalize; terminal directo y JSON sin sanear rechazados; redacción de secretos. | Aplicar redacción a futuros adaptadores. |
| GAP-012 | P1 | Abierto | Forge/release está corregido solo en prompts y shell. | Implementar puerto Forge, adaptador GitHub y release reconciliable. |
| GAP-013 | P1 | Cerrado | `sddk-testkit::TestRepository` ofrece fixture reutilizable con aislamiento de paths. | Extenderlo cuando storage y capabilities requieran harness compartido. |
| GAP-014 | P2 | Abierto | Vault, FTS5, backlinks y petgraph no están iniciados. | Mantener PR8 detrás de PR5-PR7. |
| GAP-015 | P2 | Abierto | Distribución, receipts, SBOM y attestations no están iniciados. | Mantener PR9 como último corte. |

## Detalle de bloqueos P0

### GAP-001 — Base entregada y versionada

La base fundacional se integró en `main` mediante PR #2 y se publicó como `v0.1.0`. `target/` permanece excluido.

El corte `v0.2.0` se integró mediante PR #4, trazado por issue #3 y verificado con el mismo flujo trunk-based.

**Criterio de cierre:** commits separados por comportamiento, revisión de archivos incluidos, `target/` excluido, CI verde y trazabilidad entre cada commit y las historias cerradas.

### GAP-002 — Gates de CI implementados

`.github/workflows/ci.yml` define un único check obligatorio y reproducible sobre Rust 1.91.0, el MSRV real de las dependencias bloqueadas.

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

### GAP-003 — Rust es la autoridad operativa local

`crates/sddk-cli/src/lib.rs` expone:

- `project resolve`;
- `adopt plan|apply|status|repair`;
- `lint`;
- `generate docs|inventory`;
- `cycle start|status|transition|rebuild`;
- `cycle lock acquire|release|status`;
- `ledger verify|events`.

El flujo local (adopción → ciclo → fases → ledger → rebuild) está controlado por el CLI Rust con timestamps y actores explícitos. Quedan fuera de esta unidad: capabilities, vault, reconcile y release.

**Criterio de cierre:** un test end-to-end recorre CLI → Engine → Storage creando ciclo, completando fase, aplicando fencing y reconstruyendo estado desde el ledger.

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

### GAP-005 — Frontera de capacidades implementada

ADR-0005 exige `validate → plan → authorize → apply → verify → receipt`. `sddk-gateway` cubre:

- policy default-deny derivada de `forge.capabilities` del workflow;
- approvals R3/R4: `modifies`/`irreversible` o riesgo `high`/`critical` exigen `--approve`;
- runner con argv separado, environment allowlist, timeout y truncado (sin shell);
- filesystem `ScopedFs` con rechazo de escapes, parents y symlinks, y escritura atómica;
- receipts con lifecycle `started → succeeded|failed` (begin/finalize), idempotencia por clave y redacción de secretos.

Quedan fuera: Git local (SDDK-603) y CAS (SDDK-604), que se construirán sobre este gateway antes de habilitar efectos externos.

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
| PR 4 | SQLite, hash chain, engine, replay, leases y CLI | Completo; autoridad local probada extremo a extremo. |
| PR 5 | Gateway default-deny, Git local con postcondiciones y CAS SHA-256 | Completo y probado. |
| PR 6 | Schema validation runtime, adaptador legacy y permisos por fase | Completo y probado. |
| PR 7 | Contrato legacy de release con tests | Parcial; Forge/reconcile runtime no iniciados. |
| PR 8 | ADRs y templates | No iniciado. |
| PR 9 | ADR de distribución | No iniciado. |

## Verificación ejecutada

| Gate | Resultado |
| --- | --- |
| `cargo test --workspace --locked` | PASS, 133 tests en el corte. |
| `sddk lint --format json` | PASS, 0 errores y 0 warnings. |
| `sddk generate docs --check` | PASS, documentación actual. |
| `sddk generate inventory --check` | PASS, 64 agentes y 90 skills. |
| `tests/test_workflow_contract.sh` | PASS, 117 checks. |
| `tests/test_adoption_contract.sh` | PASS, 22 checks. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS (MSRV 1.91). |
| E2E CLI `cli_validate_agent_result_and_legacy_conversion` | PASS, validación y adaptador con warnings. |
| E2E CLI `cli_permission_policy_enforces_default_deny` | PASS, default-deny y gate en capability apply. |
| Gateway (policy, runner, filesystem, Git, CAS, permissions, redacción) | PASS, 28 tests. |
| CI remota | PASS en [`Required quality gates`](https://github.com/Rubentxu/sddk-framework/actions/runs/30888909675), 53 s. |

## Plan de acción recomendado

### Work unit A — Canon y baseline

**Estado:** completado en `v0.2.0`.

**Acciones:**

1. Resolver UUID vs `hostname-path`.
2. Declarar root workflow/schemas como autoridad única.
3. Marcar o retirar snapshots incompatibles del paquete.
4. Corregir `.gitignore` para excluir `target/`.
5. Separar commits PR1, PR2, PR3 y PR4 por comportamiento.

**Gate:** checkout limpio reproduce todos los gates locales.

### Work unit B — CI y testkit

**Estado:** completado en `v0.2.0`.

**Acciones:** CI mínima, testkit con fixture repository reutilizable, Clippy estricto y checks locked.

**Gate:** required check único o conjunto documentado, obligatorio antes de merge.

### Work unit C — Cierre PR4

**Estado:** completado en `v0.3.0`.

**Acciones:** CLI cycle/phase/ledger, frame invariant, leases/fencing, replay rebuild y errores estables.

**Gate:** test end-to-end sin red ni reloj real que recorra un ciclo y reconstruya estado.

### Work unit D — Capability gateway

**Estado:** completado en `v0.4.0`.

**Objetivo:** crear una frontera no eludible antes de Git/Forge.

**Acciones:** policy default-deny, approvals vinculadas al plan, begin/finalize/reconcile receipt, runner tipado, filesystem seguro y redacción.

**Gate:** pruebas negativas demuestran que R3/R4, shell arbitrario, path escape y gate autoafirmado son rechazados.

### Work unit E — Git, CAS y agentes

**Estado:** Git local y CAS completados en `v0.5.0`; adaptador legacy y permisos por fase quedan para PR6.

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
| Fencing de mutaciones | Cerrada | Transición exige owner+fencing token cuando el ciclo está leaseado; lease expirado re-acquire con token incrementado. |
| Approvals R3/R4 | Cerrada | `modifies`/`irreversible` o riesgo high/critical exigen `--approve` explícito; desconocidas → denied. |
| Lifecycle de receipts | Cerrada | `started → succeeded|failed` vía begin/finalize; terminal directo rechazado; request/result redactados. |
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
