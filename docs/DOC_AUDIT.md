# Auditoría de documentación del árbol integrado

**Corte del 18 de noviembre de 2026.** Se contrastaron el HEAD remoto y un checkout limpio de `main` en `dbcd8eeba183b1a8af7722e706c2bfa234206efc`. El árbol documentado se reconstruyó aplicando primero el parche de horn-simulation y luego el de Wavebox. Se comprobó la correspondencia con el diff del árbol local antes de editar. Las revisiones externas están en las [decisiones sobre bocinas](horn-simulation-integration/DECISIONES.md) y los [modelos de red](wavebox-integration/MODELOS.md).

## Inventario y hallazgos

[Arquitectura](ARCHITECTURE.md) identifica capacidad, propietario, API, propósito, recorrido ejecutable y evidencia. Las tablas siguientes completan ese inventario con la clasificación del hallazgo y el destino canónico. La [guía de escenarios](INTEGRATION.md#recorridos-por-escenario) distingue ejemplo de aplicación de fixture de prueba.

| Subsistema y documento examinado | Clasificación | Corrección y referencia mantenida |
| --- | --- | --- |
| Geometría, README, roadmap y Validación | Contradicción e información desactualizada | Se distinguen geometría local de tractrix/OS, ley Hypex y trabajo futuro. [Geometría](horn-simulation-integration/API.md). |
| Webster y radiación en Algoritmos | Documentación incorrecta | Área circular con π, ruta uniforme cerrada, cámara frontal en paralelo y escala de campo según ángulo sólido. [Contrato](ALGORITHMS.md). |
| Potencia, presión y fase en Integración | Contradicción | El resumen legado indica absorción de cargas y fase reducida. En OD interviene la suma de salidas. [Magnitudes](ALGORITHMS.md#convenciones-de-entrada-y-observación). |
| Impedancia de motor y redes | Ausencia de documentación transversal | `System` incluye Rg en la Ze del punto resumen. `Driver::input_impedance` y la red diferencial informan impedancia en bornes. [Conexión con circuitos](ARCHITECTURE.md#dependencias-y-conversiones). |
| Bandpass en Algoritmos y Validación | Afirmación científica sin evidencia | El corte a 1 kHz de [bp_production](../tests/bp_production.rs) se aplica también a Ze y Xd. Se retiró la garantía fuera de banda. |
| TH y bandpass en tablas repetidas | Duplicidad e información histórica presentada como vigente | [Validación](VALIDATION.md) conserva la referencia por caso. El RMS histórico de Xd puede usar el total de filas. El informe correctivo documenta el otro denominador. |
| Medición multivía en Rustdoc | Documentación incorrecta | [validate_curves](../crates/speakerbox-measure/src/validate.rs) evalúa magnitud y **fase circular**. Distancia y tensión son metadatos. |
| Crossovers y multivía en la portada de circuits | Información desactualizada | La crate acústica ya suma vías. El circuito entrega tensión y admite EQ analógica. [Responsabilidades](ARCHITECTURE.md#dependencias-y-conversiones). |
| Red, pérdidas, procesamiento y optimización | Ausencia de documentación en el recorrido normal | Se incorporaron propietario, entradas, límites y pruebas al [inventario](ARCHITECTURE.md) y a [Integración](INTEGRATION.md). |
| Banco, importación y multivía en la guía | Ejemplo no compilable o dependiente de archivos externos | Bloques autocontenidos con driver explícito, WDR sintético y tratamiento del resultado de la suma. Los 13 bloques Rust se ejecutan en CI. |
| Enlaces relativos desde las páginas HTML de Rustdoc | Enlace roto fuera del árbol fuente | Los módulos multivía y de validación usan referencias Rustdoc a tipos y módulos. El índice Markdown conserva los contratos del repositorio. |
| Guías de agentes y README | Instrucción ambigua y duplicidad | [Archivo raíz](../AGENTS.md) para invariantes y comandos, [guía operativa](AGENTS.md) para impacto y promoción de modelos. Se conserva íntegro el bloque administrado de configuración local. |

**Límites que siguen abiertos.** BPB supera el umbral de excursión. ABC y BPC conservan fallos puntuales. El compilador puede verificar tipos y ejemplos, pero una prueba de suma con FRD sintéticos no establece validación física multivía. El ejemplo `compare_th` y la prueba BP calculan el RMS legado de Xd con el denominador histórico. La métrica puede resultar menor que si se divide solo por las filas comparables. Esta edición documenta la diferencia. Para resultados cuantitativos use siempre la banda y el denominador de la referencia elegida.

## Verificación reproducible

Desde la raíz con Rust estable, la cadena de [comandos obligatorios](../AGENTS.md) pasó en el árbol documentado. Se ejecutaron 417 pruebas en `cargo test --workspace` (incluye diez doctests). La ejecución separada de `cargo test --doc --workspace` repitió esos diez. Pasaron formato, Clippy en todos los targets, Rustdoc con advertencias como error, `cargo +1.75.0 check --workspace --all-targets` y la compilación de las tres bibliotecas para `wasm32-unknown-unknown`. Esta última verifica compilación, sin ejecutar un navegador.

- **Enlaces y recetas.** Después de `cargo build --workspace`, ejecute `python3 tools/check-docs.py`. Recorre los enlaces Markdown relativos del raíz y de docs, comprueba destinos y anclas de encabezados y ejecuta los bloques Rust de Integración como un consumidor externo. No requiere paquetes Python. El alcance del comprobador excluye URLs remotas, rutas de ejemplo escritas como texto y configuraciones de herramientas.
- **Ejemplos.** Se compilaron y ejecutaron los 12 ejemplos de la raíz, los dos de circuits y el de measure en release. `compare_oracle` usó el registro y TSV por defecto. `import_driver` usó el texto WDR sintético de [su prueba](../tests/driver_import.rs). Para `validate_multiway` se usaron seis frecuencias de 100 a 3200 Hz, dos vías idénticas a 0 dB y 0°, y suma a 6,0206 dB y 0°, a 1 m y 2,83 V RMS. El catálogo de [ejecutables](INTEGRATION.md#ejecutables-y-entradas) indica cuándo aportar archivos.
- **API pública.** Se generó Rustdoc de las tres crates y se inspeccionaron las ubicaciones señaladas por `RUSTDOCFLAGS="-W missing-docs" cargo doc --workspace --no-deps`. Se priorizaron métodos y datos de interpretación física. El lint opcional sigue identificando miembros sin comentario individual, entre ellos campos de diagnóstico. No es una puerta de aceptación de cobertura por recuento.
- **Cambios.** README, índice y arquitectura orientan al lector. Integración, Algoritmos, Validación, formato de registros y roadmap fijan uso, madurez y límites. AGENTS raíz y detallado distribuyen responsabilidades. Los cambios en `.rs` corresponden a comentarios. El control de enlaces y ejemplos se añade al job documental de la CI existente. Los registros de pruebas, modelos y calibraciones se conservan.

## Dos recorridos de comprensión

**Desarrollador.** Desde README, seleccionar la crate acústica y ejecutar `cargo run --example quickstart`. Para la caja sellada de 50 L a 50 Hz se obtuvieron 83,50 dB, 29,15 Ω y 0,639 mm de pico. Se aclaró que el cero impreso en la columna de puerto representa una salida no aplicable en ese ejemplo. La ruta de errores conduce a `try_response_at`.

**Agente.** Desde AGENTS, localizar el constructor PH1 en [network/topology.rs](../src/network/topology.rs), verificar ambas caras y el colector en [Arquitectura](ARCHITECTURE.md) y ejecutar `cargo test --test acoustic_network --test wavebox_hornresp`. La prueba de conectividad y la comparación de excursión miden propiedades distintas. Se hizo explícita la ruta entre ambas y el requisito de oráculo propio para promover un diseño.
