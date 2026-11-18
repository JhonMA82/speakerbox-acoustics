# Resultados y rendimiento

La base fue `main` en `dbcd8ee` con el parche previo de horn-simulation
aplicado. Pasaron 398 pruebas antes de los cambios. La suite final ejecutó
417 pruebas en depuración y release, y con Rust 1.75. Se verificaron Rustfmt,
Clippy con `-D warnings`, rustdoc con `-D warnings`, objetivos nativos
y compilación de librerías del workspace a wasm32. Los 28 tests de producción
Hornresp y energía pasaron con las métricas anteriores. Los ocho fixtures
CSV se regeneraron de forma idéntica en el entorno fijado.

## Coste de las rutas existentes

Linux x86-64, AMD EPYC 9V74, CPU lógica 2, Rust 1.100.0, release con LTO
y una unidad de generación. Se usaron directorios de compilación distintos,
un calentamiento y siete pares alternados. La última columna es la mediana
de cada cociente después/antes.

| Barrido de 2,000 puntos | Mediana base, ms | Mediana final, ms | Cambio pareado |
| --- | ---: | ---: | ---: |
| System, bocina de cuatro tramos | 31.203 | 31.934 | −0.60 % |
| TappedHorn | 61.470 | 58.479 | +0.32 % |
| BP6 serie | 1.441 | 1.299 | −6.66 % |
| System, 100 cambios de volumen con 50 puntos cada uno | 77.051 | 76.861 | −0.72 % |

Se observó variabilidad de tiempo entre ejecuciones. La integración no
cambia los algoritmos usados en los barridos legados.

## Coste de la red

| Barrido de 2,000 puntos, 10–1,000 Hz | Mediana de siete ejecuciones |
| --- | ---: |
| BP4 especializado, solución compleja e informe energético | 1.90 ms |
| BP4 con red genérica, solución compleja e informe energético | 2.70 ms |
| PH1 con red genérica, solución compleja e informe energético | 113.14 ms |

La red es una herramienta adicional. Para el BP4 simple medido, el coste
extra de ensamblar y factorizar una matriz pequeña fue aproximadamente
0.80 ms por barrido. El tiempo PH1 incluye siete tramos y su propagación.
Los datos crudos están en [benchmarks.csv](benchmarks.csv).

```bash
cargo build --locked --release --example perf_sweep --example wavebox_perf
taskset -c 2 target/release/examples/perf_sweep
taskset -c 2 target/release/examples/wavebox_perf
```

## Aplicación del parche

El cambio Wavebox es incremental sobre el parche de horn-simulation.
Sobre `main` en `dbcd8ee`, aplique primero `speakerbox-horn-simulation.patch`.
Si ya tiene ese contenido, puede comprobar y aplicar el parche Wavebox
directamente con `git apply --check` y `git apply`. Se verificó en un
checkout limpio con el contenido previo aplicado.
