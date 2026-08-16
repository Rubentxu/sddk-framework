# Deep Research Integration — Bundle standalone en SDDK

## ¿Qué es?

22 skills `deep-*` + 1 agente ejecutor (`deep-research-orchestrator`) + 1 workflow (`sddk-b-research`). Marco metodológico de Donella Meadows como lente transversal para investigar CUALQUIER tema.

## Componentes

```
sddk-framework/
├── agents/deep-research-orchestrator.md      (ejecutor)
├── skills/
│   ├── DEEP-RESEARCH-INDEX.md                 (catálogo)
│   ├── deep-research-orchestrator/            (gate)
│   ├── deep-research-methodology-hub/         (hub renombrado)
│   └── deep-{19 más}/                         (toolkit)
├── workflows/sddk-b-research/
│   ├── WORKFLOW.yaml                          (workflow ejecutable)
│   └── references/                            (sub-workflows)
├── docs/
│   ├── deep-research-integration.md           (este doc)
│   └── skill-categorization.md
└── docs/sddk-2.0-architecture-consolidation/
    ├── adrs/ADR-019-workflow-self-discovery.md (workflows autodiscovery)
    └── adrs/ADR-0016-skill-namespace-categorization.md (skill categorization)
```

## Pipeline R0-R6 (Meadows como lente)

```
R0  Definir el sistema del tema (Meadows) [obligatorio]
R1  Build agenda (deep-research-strategist)
R2  Discover sources (deep-source-discovery-specialist)
R3  Evaluar credibilidad + validar refs (en paralelo)
R4  Triangular (deep-evidence-triangulator)
R5  Consolidar corpus (deep-knowledge-corpus-curator)
R6  Extraer deliverables (deep-claim-extractor)
```

## Sub-pipelines

Software research, Pattern extraction, Domain modeling, Knowledge graph, Historical lineage, Scenarios, Paradigms, Traps, **Systems Thinking** (Meadows).

## Anti-patrones (Meadows labels)

- Saltarse R0 = "collecting data without a lens"
- Confundir L3 con L1 = Shifting the Burden
- Single-source critical = Insufficient triangulation
- Inventar cuantificaciones = Seeking the Wrong Goal
- Cambiar parámetros cuando el problema es de paradigma = también Seeking the Wrong Goal

## Distribución

`sddk dev install` publica el bundle runtime completo. `sddk dev link` lo enlaza a ZCode y OpenCode. Todo lo que esté en `skills/`, `agents/`, `workflows/`, `docs/` se distribuye automáticamente.

## Estado

Versión 1.0 · Standalone · Patrón parity con skills (directorios, references/, assets/, _shared/).
