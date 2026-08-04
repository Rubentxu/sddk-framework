# SDDK v3.6 Stabilization Package

Paquete de diseño y ejecución para transformar SDDK Framework en un motor de desarrollo dirigido por especificaciones, determinista, trazable y compatible con una base de conocimiento tipo Obsidian.

## Contenido

- `PRD.md`: requisitos del producto y criterios de éxito.
- `ROADMAP.md`: secuencia de PRs y gates de entrega.
- `BACKLOG.md`: épicas, historias y criterios de aceptación.
- `CURRENT-STATE-AUDIT.md`: estado verificado, gaps, desviaciones y orden de remediación.
- `MIGRATION.md`: transición desde los agentes y prompts actuales.
- `adr/`: decisiones arquitectónicas propuestas.
- `workflow/workflow.yaml`: snapshot histórico del contrato inicial; no es la fuente runtime actual.
- `schemas/`: snapshots históricos de contratos iniciales; pueden divergir de los schemas raíz.

## Estado actual

El paquete está en implementación y conserva snapshots de diseño que ya divergen de los contratos ejecutables raíz. Consulta [`CURRENT-STATE-AUDIT.md`](CURRENT-STATE-AUDIT.md) antes de cerrar historias o usar los ficheros bajo `workflow/` y `schemas/` como fuente runtime.

Las fuentes ejecutables actuales son:

- `../../workflow/workflow.yaml`;
- `../../schemas/*.json`;
- los crates bajo `../../crates/`.

## Decisiones principales

1. Los agentes proponen; el runtime Rust valida y ejecuta.
2. Markdown + frontmatter es la fuente canónica del conocimiento.
3. SQLite almacena el ledger, el estado operativo y el índice del vault.
4. Los artefactos grandes se almacenan por hash en el filesystem.
5. Los packs se declaran mediante manifiestos y comienzan compilados en el binario.
6. LadybugDB se aplaza hasta que existan requisitos y benchmarks que justifiquen su coste.
7. El workflow se define una sola vez y la documentación se genera desde contratos estructurados.

## Destino objetivo original

```text
docs/product/PRD-SDDK-v3.6.md
docs/architecture/adr/ADR-0001-*.md
...
docs/delivery/ROADMAP-v3.6.md
docs/delivery/BACKLOG-v3.6.md
docs/migration/MIGRATION-v3.5-v3.6.md
workflow/workflow.yaml
schemas/agent-result.schema.json
schemas/capability-request.schema.json
```

Esta estructura conserva la propuesta original y no representa por sí sola el layout runtime actual. No se recomienda modificar todavía todos los agentes. Primero deben consolidarse el canon raíz, la CI, la identidad de proyecto y el ledger.
