# Golden Dataset — Meta-verificación SDDK

Conjunto de casos de prueba con **verdicts conocidos** para medir si los clusters de debt-verify detectan deuda real o solo producen YAML plausible.

## Propósito

Sin este dataset, no hay forma de saber si `debt-smells-cluster` realmente detecta un god-class o si solo formatea findings que parecen correctos. El golden dataset es la **verificación de los verificadores**.

## Estructura

```
golden-dataset/
├── cases/
│   ├── 01-clean-pass/              ← cambio limpio → PASS esperado
│   │   ├── spec.md                 ← spec del cambio
│   │   ├── implementation/         ← código (2-5 archivos)
│   │   ├── expected-verdict.yaml   ← verdict + findings esperados
│   │   └── known-issues.md         ← falsos positivos conocidos (qué NO debería encontrar)
│   ├── 06-god-class-fail/          ← god-class obvio → FAIL esperado
│   ├── 11-circular-import-fail/    ← circular import → FAIL esperado
│   ├── 16-subtle-feature-envy-pw/  ← deuda sutil → PW esperado
│   └── 21-adversarial-hidden-mutation/ ← parece limpio, tiene issue → FAIL esperado
├── runner/
│   └── run-golden.sh               ← ejecuta los clusters contra cada caso
└── results/                        ← salida del runner (TP/FP/FN/TN)
```

## Buckets

| Bucket | Casos | Verdict esperado | Mide |
|---|---|---|---|
| **Limpios** | 01-05 | PASS | Falsos positivos (¿marca debt donde no lo hay?) |
| **Debt crítico obvio** | 06-10 | FAIL | Falsos negativos graves |
| **Deuda sutil** | 11-15 | PASS_WITH_WARNINGS | Sensibilidad |
| **Adversariales** | 16-20 | Variable | Robustez (código que engaña) |

## Cómo ejecutar

```bash
cd ~/Proyectos/agentesIA/sddk-framework/golden-dataset
./runner/run-golden.sh                    # corre todos los casos
./runner/run-golden.sh cases/06-god-class-fail/  # corre un caso específico
```

## Cómo añadir un caso

1. Crear `cases/NN-descripción/`
2. Escribir `spec.md` (qué se supone que hace el cambio)
3. Escribir `implementation/` (el código a auditar — TypeScript por defecto)
4. Escribir `expected-verdict.yaml` (verdict esperado + findings clave)
5. Escribir `known-issues.md` (falsos positivos a vigilar)

## Métricas que produce

- **Precision** = TP / (TP + FP) — de lo que marca como debt, ¿cuánto lo es?
- **Recall** = TP / (TP + FN) — del debt real, ¿cuánto detecta?
- **F1** = 2 × (precision × recall) / (precision + recall)

Objetivo: precision > 0.8, recall > 0.7. Por debajo de eso, los clusters necesitan ajuste.

## Estado actual

Solo 5 casos iniciales (1 por bucket) como prueba de concepto. El dataset debe crecer a 15-20 para ser estadísticamente útil.
