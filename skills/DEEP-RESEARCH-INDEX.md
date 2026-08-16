# Deep Research Skills — Sistema de investigación profunda

**22 skills** + 1 agente ejecutor. Marco metodológico de Donella Meadows como lente transversal.

> Taxonomía metadata: `category: deep-research` + `subcategory: <gate|methodology-hub|r-pipeline|domain-pipeline|systems-thinking>`. Ver `docs/skill-categorization.md`.

## Las 22 skills

### Gate (orchestrator-side) — 1
| # | Skill | Función |
|---|-------|---------|
| 1 | `deep-research-orchestrator` | SKILL con gate; el orchestrator la carga y delega |

### Methodology Hub — 1
| # | Skill | Función |
|---|-------|---------|
| 2 | `deep-research-methodology-hub` | Hub metodológico (renombrado); el agente la carga |

### R-Pipeline core (R1-R6) — 7
| # | Skill | Fase |
|---|-------|------|
| 3 | `deep-research-strategist` | R1 |
| 4 | `deep-source-discovery-specialist` | R2 |
| 5 | `deep-source-credibility-assessor` | R3a |
| 6 | `deep-reference-validator` | R3b |
| 7 | `deep-evidence-triangulator` | R4 |
| 8 | `deep-knowledge-corpus-curator` | R5 |
| 9 | `deep-claim-extractor` | R6 |

### Domain pipelines — 6
| # | Skill | Cuándo |
|---|-------|--------|
| 10 | `deep-software-research` | Tecnología |
| 11 | `deep-pattern-extractor` | Code patterns |
| 12 | `deep-domain-modeler` | Entities |
| 13 | `deep-knowledge-graph-builder` | Relations |
| 14 | `deep-historical-lineage-tracer` | Temporal |
| 15 | `deep-scenarios-explorer` | Future |

### Systems Thinking (Meadows) — 7
| # | Skill |
|---|-------|
| 16 | `deep-coach-systems-thinking` |
| 17 | `deep-leverage-points-analyst` |
| 18 | `deep-system-archetypes-mapper` |
| 19 | `deep-feedback-loops-analyzer` |
| 20 | `deep-stocks-flows-diagrammer` |
| 21 | `deep-paradigms-explorer` |
| 22 | `deep-traps-detector` |

## Pipeline R

```
R0  Definir sistema (Meadows)         [obligatorio]
R1  Build agenda
R2  Discover sources
R3  Evaluar credibilidad + validar refs (paralelo)
R4  Triangular
R5  Consolidar corpus
R6  Extraer deliverables
```

## Estado

v1.0 · Standalone · Workflow: `workflows/sddk-b-research/WORKFLOW.yaml`
