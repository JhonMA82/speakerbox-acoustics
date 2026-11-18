# Documentación del workspace

| Objetivo | Entrada | Resultado esperado |
| --- | --- | --- |
| Ejecutar una caja por primera vez | [Inicio rápido](../README.md) | Una fila por diseño a 50 Hz |
| Integrar el motor o interpretar una salida | [Integración](INTEGRATION.md) | Diseño validado, observables y diagnósticos |
| Elegir una crate o un módulo | [Arquitectura e inventario](ARCHITECTURE.md) | Propietario de tipos, dependencias y pruebas |
| Consultar unidades, signos y modelos | [Algoritmos](ALGORITHMS.md) | Convención común y excepciones por topología |
| Saber qué se comparó | [Validación](VALIDATION.md) | Variable, fixture, banda y criterio |
| Conservar un diseño | [Registros y esquemas](RECORD_FORMAT.md) | Formato y subconjunto importable |
| Preparar una modificación | [Guía del agente](AGENTS.md) | Contratos afectados y pruebas a ejecutar |
| Auditar la documentación | [Hallazgos y reproducción](DOC_AUDIT.md) | Diferencias corregidas, controles y límites |

## Revisión y compatibilidad

Esta edición describe el árbol local compuesto de `main@dbcd8eeba183b1a8af7722e706c2bfa234206efc` y los cambios de [perfiles y propagación](horn-simulation-integration/README.md), seguidos de los de [redes](wavebox-integration/README.md). Al comprobar el remoto el 18 de noviembre de 2026, esos cambios eran locales. Una aplicación que dependa solo de aquel SHA carece de las APIs de ambos parches. Las versiones de los manifiestos son las mismas en los dos árboles. Fije la revisión de su dependencia.

Los tres manifiestos declaran Rust 1.75, edición 2021, sin features opcionales declaradas. La guía usa la raíz del workspace como directorio actual. Para las bibliotecas WASM, pase datos desde el host mediante texto o bytes. La lectura de drivers por ruta se excluye en wasm32. Los ejemplos con ficheros, las herramientas Python de comparación y la reproducción con software externo son operaciones de desarrollo en un host. Consulte las [comprobaciones](DOC_AUDIT.md) para distinguir compilación de ejecución en navegador.

## Archivos de evidencia

| Colección | Estado y uso |
| --- | --- |
| [Auditorías físicas](audits/) | Históricas por revisión. Siguen vigentes los contratos específicos de [radiación](audits/RADIATION_MODEL_CONTRACT.md) y [suma multivía](audits/MULTIWAY_PRESSURE_CONTRACT.md) en el alcance descrito en los manuales. |
| [Integración de referencia](reference-integration/) | Evidencia por experimento sobre líneas, datos, optimización y pérdidas. Reproducir antes de trasladar tiempos a otro equipo. |
| [Comparación de bocinas](horn-simulation-integration/VALIDACION.md) y [comparación de redes](wavebox-integration/VALIDACION.md) | Condiciones y límites de los fixtures incorporados. |
| [Historial](../CHANGELOG.md), [planes](../ROADMAP.md), [criterios de revisión](../REVIEW.md) e [informes de estabilización](../LIBRARY_STABILIZATION_REPORT.md) | Fechas, decisiones y alcance de cada entrega. El catálogo vigente está en Arquitectura. |
| [Identificación](../SPEAKERWORKSHOP_IDENTIFICATION_REPORT.md), [circuitos](../SPEAKERWORKSHOP_CROSSOVER_REPORT.md) e [integración SpeakerWorkshop](../SPEAKERWORKSHOP_INTEGRATION_REPORT.md) | Evidencia histórica de las crates auxiliares. |
| [FEM](horn-simulation-integration/FEM.md) | Propuesta de intercambio y criterios de adopción. El workspace carece de backend FEM ejecutable. |
