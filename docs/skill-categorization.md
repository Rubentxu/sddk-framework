# Skill Categorization — Taxonomía lógica

## Problema

El framework SDDK distribuye skills en `skills/<name>/` (un nivel). Cuando un dominio crece a 20+ skills relacionadas (como `deep-research` con 22), el listado plano se vuelve difícil de navegar.

## Solución: categorización vía metadata

Usamos **metadata en el frontmatter** (`category` + `subcategory`) para declarar la categoría. Esto es compatible con el CLI actual y permite filtrado lógico.

## Taxonomía (categoría: `deep-research`)

| Subcategoría | Skills |
|--------------|--------|
| **gate** | `deep-research-orchestrator` |
| **methodology-hub** | `deep-research-methodology-hub` |
| **r-pipeline** | `deep-research-strategist`, `deep-source-discovery-specialist`, `deep-source-credibility-assessor`, `deep-reference-validator`, `deep-evidence-triangulator`, `deep-knowledge-corpus-curator`, `deep-claim-extractor` |
| **domain-pipeline** | `deep-software-research`, `deep-pattern-extractor`, `deep-domain-modeler`, `deep-knowledge-graph-builder`, `deep-historical-lineage-tracer`, `deep-scenarios-explorer` |
| **systems-thinking** | `deep-coach-systems-thinking`, `deep-leverage-points-analyst`, `deep-system-archetypes-mapper`, `deep-feedback-loops-analyzer`, `deep-stocks-flows-diagrammer`, `deep-paradigms-explorer`, `deep-traps-detector` |

## Limitación reconocida

Agrupación física (`skills/deep-research/{21}/`) requiere modificar el CLI (single-level walker). Ver [ADR-019](../sddk-2.0-architecture-consolidation/adrs/ADR-019-workflow-self-discovery.md) y [ADR-0016](../sddk-2.0-architecture-consolidation/adrs/ADR-0016-skill-namespace-categorization.md) para la propuesta formal.

## Estado

22 skills con `category: deep-research` aplicado. Sin cambios al CLI.
