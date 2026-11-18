# speakerbox-acoustics

Workspace Rust para calcular la respuesta lineal de altavoces y recintos en frecuencia. A partir de un driver, la geometría, el aire y la excitación se obtienen impedancia, excursión y, cuando el modelo define un observador, presión sonora. La comparación con Hornresp cubre diseños y variables concretos. La distorsión, la compresión térmica y el campo tridimensional quedan fuera del cálculo.

| Si necesita… | Empiece en |
| --- | --- |
| Diseñar una caja, bocina, línea o red de conductos | [speakerbox-acoustics](docs/INTEGRATION.md) |
| Calcular la tensión aplicada por un crossover con carga compleja | [speakerbox-circuits](crates/speakerbox-circuits/src/lib.rs) |
| Leer FRD/ZMA o identificar un circuito equivalente | [speakerbox-measure](crates/speakerbox-measure/src/lib.rs) |

## Primera simulación

Desde la raíz del checkout, ejecute:

```sh
cargo run --example quickstart
```

La primera fila corresponde a una caja sellada de 50 L a 50 Hz en medio espacio. Lea el SPL a 1 m, el módulo de impedancia en Ω y la excursión de pico en mm. La última columna solo tiene significado para una salida con puerto. Abra [el ejemplo](examples/quickstart.rs) para cambiar volumen, driver o frecuencia. Para incorporar el motor a otra aplicación, use una dependencia local:

```toml
[dependencies]
speakerbox-acoustics = { path = "../speakerbox-acoustics" }
```

## Elegir la siguiente lectura

- [Índice](docs/index.md) y [arquitectura](docs/ARCHITECTURE.md) para localizar APIs, ejemplos y pruebas.
- [Guía práctica](docs/INTEGRATION.md) para seleccionar una topología, validar entradas e interpretar errores.
- [Contrato físico](docs/ALGORITHMS.md) para unidades, fase y energía. [Validación](docs/VALIDATION.md) para consultar oráculos y sus límites.
- [Guía de mantenimiento](AGENTS.md) para modificar el workspace.

La crate raíz declara 0.2.0 y las auxiliares 0.1.0. Son versiones previas a 1.0. «v1» identifica el alcance histórico de topologías con oráculo propio. [Compatibilidad y procedencia del checkout](docs/index.md).
