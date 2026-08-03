# SDDK Framework

> **Spec-Driven Development Kernel** — un workflow agéntico para ingeniería de software con grafo de conocimiento integrado, disciplina git trunk-based y verificación multi-lente.

[![Licencia: MIT](https://img.shields.io/badge/Licencia-MIT-yellow.svg)](LICENSE)
[![OKF Compatible](https://img.shields.io/badge/OKF-v0.2-blue.svg)](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
[![Obsidian Compatible](https://img.shields.io/badge/Obsidian-Properties_v1.4+-purple.svg)](https://obsidian.md/)

[English](README.md) | **[Español](README.es.md)**

---

## ¿Qué es SDDK?

SDDK es un framework completo de orquestación de agentes para desarrollo de software asistido por IA. Coordina agentes IA a través de un pipeline estructurado — desde la exploración hasta el release — con puertas de calidad integradas, auditoría de deuda técnica y un grafo de conocimiento que rastrea cada decisión, requisito e incidencia a lo largo de los ciclos.

### Diferenciadores clave

| Característica | Qué hace |
|----------------|----------|
| **Spec-Driven** | Cada cambio empieza con una spec (escenarios Given/When/Then). La implementación se verifica contra la spec, no solo "¿compila?" |
| **Verificación multi-lente** | 6 lentes paralelos (compliance de spec, arquitectura, calidad de tests, coherencia de diseño, 2 jueces adversariales) + síntesis |
| **Auditoría de deuda técnica** | 5 agentes cluster (arquitectura, smells, duplicación, coupling, over-engineering) auditan deuda antes del merge a main |
| **Grafo de conocimiento** | Cada milestone, ADR, requisito, ciclo e incidencia es un nodo en un grafo de wikilinks compatible con Obsidian. Trazabilidad bidireccional completa |
| **Garantía trunk-based** | Un ciclo no puede declarar `success` hasta que los cambios estén mergeados a `main` + tag semver + trunk sincronizado. Sin abortos silenciosos |
| **Lock de serialización** | Un ciclo a la vez. El lock sobrevive caidas de sesión |
| **Agnóstico al editor** | Funciona con ZCode y OpenCode (extensible a cualquier runner de agentes) |

## Arquitectura

```
┌─────────────────────────────────────────────────────┐
│                   ~/.sddk-shared/                    │
│           (este repositorio — framework)             │
│                                                      │
│  ┌──────────┐  ┌─────────┐  ┌────────────────────┐  │
│  │  agents/  │  │ skills/ │  │ prompts/sdd-kernel │  │
│  │  (63)     │  │ (89)    │  │ (phase specs, MCW) │  │
│  └────┬─────┘  └────┬────┘  └─────────┬──────────┘  │
│       │              │                  │             │
│  ┌────┴──────────────┴──────────────────┴──────────┐ │
│  │       knowledge-template/ (plantilla vault)      │ │
│  │  milestones · adrs · specs · cycles · incidences │ │
│  └─────────────────────────────────────────────────┘ │
│                                                      │
│  ┌─────────────┐  ┌──────────────────┐               │
│  │golden-dataset│  │ bootstrap.sh     │               │
│  │(meta-testing)│  │ (instalador)     │               │
│  └─────────────┘  └──────────────────┘               │
└─────────────────────────────────────────────────────┘
         │                                    │
    ┌────┴────┐                         ┌─────┴─────┐
    │ ZCode   │                         │ OpenCode  │
    │(symlinks)│                        │(symlinks) │
    └─────────┘                         └───────────┘
         │
    ┌────┴──────────────┐
    │ {project}/.sddk-knowledge/ │  (vault por proyecto,
    │   (committed to git)       │   creado por sddk-adopt)
    └───────────────────┘
```

## Inicio rápido

### Instalar

```bash
git clone https://github.com/Rubentxu/sddk-framework.git ~/.sddk-shared
~/.sddk-shared/bootstrap.sh --all
```

El script de bootstrap detecta automáticamente los editores instalados (ZCode, OpenCode) y crea los symlinks. Tus repos de proyecto quedan limpios — **cero archivos de documentación en tus repos de código**.

### Ejecutar un ciclo

**¿Primera vez en un proyecto?** Adóptalo primero:

```bash
cd tu-proyecto
/sddk-adopt         # una vez: auditar proyecto, plantar artefactos SDDK, crear vault de conocimiento
/sddk-init          # una vez: detectar stack, testing, modo TDD
/sddk-new add-auth  # iniciar un ciclo SDDK completo
```

**Ciclos posteriores** (proyecto ya adoptado):

```bash
cd tu-proyecto
/sddk-new <change-name>  # el vault .sddk-knowledge/ ya existe; init se omite
```

`sddk-adopt` crea un **stamp de adopción único** (`.sddk-knowledge/.adopted`) para que futuras invocaciones de `sddk-init` salten el chequeo pesado de adopción y vayan directo al refresh de contexto.

El orchestrator ejecutará:
1. **Planificación** — explore → propose → spec → design → tasks (con checkpoints interactivos)
2. **Construcción** — apply (con Strict TDD si activado) → verify (multi-lente) → debt-verify (5 clusters)
3. **Release** — push → PR → merge a main → tag semver → actualizar grafo de conocimiento → sincronizar trunk

Ningún ciclo se cierra hasta que tu código está en `main`.

## Paths del workflow

| Path | Cuándo | Profundidad |
|------|--------|-------------|
| **B-direct** | Hotfix, tarea acotada | Cargar skill → ejecutar → verify ligero → release |
| **A-min** | Cambio simple, contexto C2 | spec → apply → verify → debt-verify (smoke, 2 clusters) → release |
| **A-lite** | Trabajo acotado, contexto C1 | propose → spec → apply → verify → debt-verify (standard, 4 clusters) → release |
| **A-full** | Arquitectura, dominio nuevo, C0 | explore → propose → spec ∥ design → tasks → apply → verify (6 lentes) → debt-verify (deep, 5 clusters) → release |

El eje de reversibilidad (v3.4) modula la profundidad del debt-verify independientemente:
- **Alta reversibilidad** (código puro, feature flag) → saltar debt-verify
- **Baja reversibilidad** (schema, seguridad) → forzar deep + judgment-day

## Grafo de conocimiento

Cada ciclo puebla un vault de conocimiento en `{project}/.sddk-knowledge/` (dentro del repo, commited):

```
mi-app/.sddk-knowledge/
├── _index.md              ← MOC con queries Dataview
├── milestones/
│   ├── _active.md         ← lock de serialización
│   └── M-001-auth.md      ← [[ADR-003]], [[REQ-Session]]
├── adrs/
│   └── ADR-003-jwt.md     ← [[REQ-Session]], log de implementación
├── specs/auth/
│   └── REQ-Session.md     ← [[ADR-003]], tested_by, verified_in_cycle
├── cycles/
│   └── CYC-2026-08-03.md  ← hub de trazabilidad (linkea todo)
├── incidences/
│   └── INC-001-lag.md     ← [[ADR-003]], afecta [[REQ-Session]]
└── terms/
    └── TERM-JWT.md
```

Ábrelo en [Obsidian](https://obsidian.md) para graph view, backlinks y queries Dataview. Basado en el [spec OKF de Google](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) con changelogs bi-temporales.

## Sistema de verificación

### Verificación funcional (`sddk-verify`)

La **Behavioral Compliance Matrix** mapea cada escenario de spec a un test que pasó en runtime. El análisis estático por sí solo nunca es verificación.

| Lente | Qué verifica |
|-------|-------------|
| Spec Compliance | Cada escenario → test cubridor → PASS en runtime |
| Architecture + Connascence | Calidad de diseño, coupling, SOLID |
| Test Quality | Assertions prohibidas, ratios de mock, triangulación |
| Design Coherence | Decisiones de diseño vs implementación |
| Adversarial Judge A | Detección ciega de deficiencias |
| Adversarial Judge B | Detección ciega de deficiencias |

### Auditoría de deuda técnica (`sddk-debt-verify`)

5 agentes cluster ejecutan en paralelo (read-only sobre el código):

| Cluster | Dimensión |
|---------|-----------|
| Architecture | Connascence, SOLID, críticas Matsumoto + Khononov |
| Smells | 12 Fowler smells con señales grep-verificables → mapeo SOLID |
| Duplication | Estructural/literal/semántica + dead code |
| Coupling | Dependencias ocultas, estado global, imports circulares |
| Over-engineering | YAGNI, ledger de deuda ponytail, trayectoria de bloat |

## Estructura del proyecto

```
sddk-framework/
├── agents/                 # 63 prompts de agentes (orchestrator, ejecutores de fase, clusters, jueces)
├── skills/                 # 89 skills (knowledge-graph, sddk-*, entropy-sdd, cognicode-sdd, ...)
├── prompts/sdd-kernel/     # Phase specs, MCW, git-contract, decision-model, plantillas ADR/roadmap
├── knowledge-template/     # Plantilla de vault (6 tipos de nodo, MOCs, lock de serialización)
├── golden-dataset/         # Casos de meta-verificación (5 casos iniciales + runner)
├── bootstrap.sh            # Instalador para ZCode/OpenCode
├── README.md               # Documentación en inglés
├── README.es.md            # Esta documentación
└── LICENSE                 # MIT
```

## Conceptos clave

- **MCW (Mandatory Complete Workflow)** — la ley. 5 fases, pasos numerados, gates duros. Fuente de verdad: `prompts/sdd-kernel/mcw.md`.
- **Lock de serialización** — un ciclo a la vez. Lock file: `milestones/_active.md`. Sobrevive crashes de sesión.
- **Release Completion Guard** — el orchestrator no puede emitir `status: success` sin `HEAD == origin/main` + tag semver confirmado en remoto.
- **Zero docs en repo** — todo el conocimiento del proyecto vive en el vault, nunca en el repo git del proyecto.
- **Changelog bi-temporal** — cada nodo registra `valid_from` / `valid_to`, permitiendo queries de time-travel.

## Compatibilidad

- **Editores**: ZCode, OpenCode (extensible a cualquier runner de agentes que lea prompts markdown)
- **Formato de conocimiento**: [OKF v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md), [Obsidian Properties v1.4+](https://obsidian.md/)
- **MCPs** (opcional): CogniCode (análisis de arquitectura), Chronos (debugging time-travel), LogSeq (vault alternativo), Engram (memoria cross-session)

## Contribuir

Las contribuciones son bienvenidas. Por favor lee la arquitectura en `prompts/sdd-kernel/mcw.md` antes de proponer cambios.

## Licencia

[MIT](LICENSE) © 2026 Rubentxu
