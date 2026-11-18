# Resultados de la integración

Línea base `dbcd8eeba183b1a8af7722e706c2bfa234206efc`, con árbol
`main...origin/main` limpio antes de editar. Workspace de tres paquetes.
Se ejecutaron depuración y release en la base, cada uno con **388 pruebas
aprobadas** y cero fallos u omisiones. El chequeo de Rust 1.75 en la base
pasó con el lockfile registrado.

| Comprobación final | Resultado |
| --- | --- |
| `cargo fmt --all --check` | Aprobada |
| `cargo test --locked --workspace` | 398 aprobadas |
| `cargo test --locked --workspace --release` | 398 aprobadas |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Aprobada |
| `RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps` | Aprobada |
| `cargo +1.75.0 check --locked --workspace --all-targets` | Aprobada |
| `cargo +1.75.0 test --locked --workspace` | 398 aprobadas |
| `cargo check --locked --workspace --lib --target wasm32-unknown-unknown` | Aprobada |
| Compilación de aplicaciones y ejemplos en nativo | Incluida en los chequeos `--all-targets` |
| Oráculos de producción y regresiones energéticas con `--nocapture` | 28 aprobadas. Métricas publicadas de Hornresp idénticas a las de `main` en la precisión reportada. |
| Fixture Python regenerado y comparado en Rust | Idéntico byte a byte en el entorno fijado, comparaciones aprobadas |
| Comparador previo de Python | 2 aprobadas |

## Precisión y potencia

| Comprobación existente | Base | Integración |
| --- | --- | --- |
| Hornresp, primer diseño con 44 frecuencias | SPL RMS 1.86 dB, Ze RMS 7.2 %, Xd RMS 9.9 % | Idénticos |
| Hornresp, diseño 2 con 533 frecuencias | SPL RMS 1.94 dB, Ze RMS 6.0 %, Xd RMS 7.1 % | Idénticos |
| Hornresp, diseño 3 con 533 frecuencias | SPL RMS 1.93 dB, Ze RMS 6.2 %, Xd RMS 7.8 % | Idénticos |
| TL uniforme, cuatro puntos de balance relativos a potencia de entrada | 3.03·10⁻¹⁶ | 3.03·10⁻¹⁶ |
| TL variable con Darcy, mismos puntos de balance | 5.95·10⁻¹² | 5.95·10⁻¹² |

Las tolerancias, datos de producción y factores empíricos se conservaron.
Las comparaciones nuevas y su alcance se detallan en [VALIDACION.md](VALIDACION.md).

## Rendimiento de rutas existentes

Rust y Cargo 1.100.0, Linux x86-64, AMD EPYC 9V74, perfil release
con LTO y una unidad de generación. Se calentó cada ejecutable y
se hicieron 15 pares de ejecuciones en CPU lógica 2. Se alternó cuál
revisión iba primero. La última columna es la mediana de las razones
pareadas, con signo negativo para una reducción de tiempo.

| Caso | Mediana base, ms | Mediana final, ms | Variación pareada |
| --- | ---: | ---: | ---: |
| System horn de 4 tramos, 2,000 puntos | 55.091 | 30.667 | −43.7 % |
| TappedHorn 4 tramos, 2,000 puntos | 108.543 | 60.001 | −44.4 % |
| System con 100 cambios de volumen y 50 puntos | 136.134 | 74.825 | −44.5 % |
| TL uniforme, 2,000 puntos | 0.920 | 0.917 | −0.2 % |
| TL variable, 2,000 puntos | 68.977 | 35.094 | −48.2 % |
| TL variable con Darcy, 2,000 puntos | 209.952 | 100.456 | −51.0 % |
| BP6 serie, 2,000 puntos | 1.253 | 1.266 | +1.0 % |
| DE acústico, 5,000 evaluaciones | 11.764 | 11.769 | +1.0 % |

Las rutas con perfiles variables se benefician de evaluar cada área de sección
una vez por posición RK4 y compartirla entre los dos estados base. Se mantienen
los mismos pasos, estados y puntos de evaluación. Una función entregada en
`Flare::Custom` debe ser determinista en su posición axial. Las diferencias
de alrededor del 1 % en BP6 y DE no justifican una atribución causal en
este entorno compartido. No se modificaron esos solvers.

## Rendimiento de los perfiles nuevos

Se comparó la primera versión funcional, con inversión por bisección y
evaluaciones repetidas de área, con la versión final. El cálculo de
geometría usa Newton acotado y conserva bisección como recuperación. El
barrido comprueba impedancia finita en 500 frecuencias logarítmicas de
25 a 630 Hz, aire de la referencia. Se hicieron siete pares de ejecuciones,
con la misma afinidad y alternancia.

| Geometría | Primer barrido, ms | Barrido final, ms | Resolver y muestrear 500 secciones, ms finales |
| --- | ---: | ---: | ---: |
| Tractrix | 1,931.931 | 75.556 | 0.216 |
| Tractrix escalada | 1,912.302 | 75.678 | 0.173 |
| OS | 6.385 | 3.815 | 0.005 |
| Exponencial-cónica | 22.008 | 9.197 | 0.013 |
| Tubo sin pérdidas | 0.064 | 0.066 | 0.016 |
| Tubo con capa delgada | 0.115 | 0.115 | 0.003 |

Los tiempos del tubo corresponden al cálculo de impedancia. El coste de
integrar adicionalmente la disipación corresponde al informe de energía.
Los archivos de medición están en [benchmarks.csv](benchmarks.csv). Para
repetir las mediciones de producción, compile y ejecute cada ejemplo en
ambas revisiones, en el mismo equipo. El ejemplo nuevo se ejecuta en la
revisión que incluye los perfiles nuevos.

```bash
cargo build --locked --release --example perf_sweep --example completion_perf
taskset -c 2 target/release/examples/perf_sweep
taskset -c 2 target/release/examples/completion_perf
cargo build --locked --release --example horn_simulation_perf
taskset -c 2 target/release/examples/horn_simulation_perf
```

## Límites pendientes

Queda pendiente una referencia geométrica para JMLC auténtico, mediciones
de adaptadores y conductos para validar las predicciones fuera de la matriz
numérica, y pérdidas termoviscosas en zonas de área variable. La nueva
selección por fidelidades se probó con dos densidades de cuadrícula de
Webster. El piloto FEM, sus mediciones y casos modales quedan para el backend
externo descrito en [FEM.md](FEM.md).
