# Arquitectura y rutas de uso

Los solvers comparten elementos físicos. La elección de topología fija las condiciones de frontera y los observables que se pueden reconstruir. `Design` agrupa `System`, `TappedHorn`, `OffsetHorn`, `Bandpass` y `TransmissionLine`. La línea también expone su estado propio. `AcousticNetwork` se resuelve fuera de esa fachada.

## Del diseño a la respuesta

1. Convierta datos de interfaz a SI en la frontera de la aplicación. Construya el driver y, si procede, el equivalente del banco.
2. Construya la geometría y declare las cargas, el aire y el generador. Una cámara concentrada aporta compliancia. Una cámara distribuida de la red tiene longitud y sección efectiva.
3. Use el método estricto de la topología. Para barridos repetidos de `Design`, `prepare()` valida la configuración y presta el diseño inmutable.
4. Consulte el fasor o la salida requerida. El punto resumen pierde, entre otros detalles, la identidad de los puertos. La energía y el campo tienen contratos distintos. [Contrato](ALGORITHMS.md).

En redes, `NetworkLayout` asigna ambas caras del driver. Cada tramo aporta una ecuación de transferencia y caudales a las filas de continuidad. La resolución mixta entrega presiones, caudales de tramos y fronteras. `solve_driver` refleja la diferencia de presión al diafragma y escala el estado con su caudal. Las aberturas explícitas alimentan el observador compacto a 1 m.

| Constructor de red | Nodos y caminos tal como se ensamblan | Alcance |
| --- | --- | --- |
| PH1 | Cerrado 0 → **1 frontal** → 2 → unión 3 → boca 4. Cerrado 5 → **6 trasero** → 7 → unión 3. | Dos caras acopladas. Comparación externa puntual. |
| DBR | Frontal 0 radia. Trasero 1 → cámara 1–2 → puerto 2–3 → cámara 3–4 → puerto 4–5 → boca 5. | Fuga opcional en la entrada de cada cámara. |
| BP8S | C1 = 0–1, C2 = 2–3, C3 = 4–5. Driver entre 0 y 2. Puertos 3–0, 1–4 y 5–6. Boca 6. | Comparado para un diseño. |
| BPB | Las mismas cámaras y caras. Puertos 1–4 y 5–6. Extremo 3 cerrado y boca 6. | Experimental por error de excursión. |
| BP8P (prueba) | Sobre BP8S, el puerto 3–0 pasa a 3–4. C1 y C2 descargan en C3 y se conserva la boca 6. | Balance en cuatro frecuencias, sin oráculo. |
| ABC (prueba) | Sobre DBR, cambia el conducto 2–3, se añade una rama de 2 a una boca nueva y fugas en 1 y 3. | Discrepancias puntuales conservadas. |
| BPC (prueba) | Sobre BPB, se añade una rama de 3 a una segunda boca y una fuga en 2. | Discrepancias puntuales conservadas. |

Estas conexiones describen nodos acústicos y condiciones de frontera. Para fabricar un recinto harían falta la geometría espacial y los detalles constructivos. Las orientaciones de tramo definen el signo del caudal. BP8P, ABC y BPC tienen configuraciones de prueba, sin constructor público específico. Los grafos exactos y las comparaciones que fallan están en [la prueba externa](../tests/wavebox_hornresp.rs). [Ensamblaje](../src/network/topology.rs) · [Ejemplo PH1 serializado](wavebox-integration/ph1-network.json) · [Madurez por variable](wavebox-integration/VALIDACION.md).

## Inventario de capacidades

La columna «prueba o recorrido» permite abrir la configuración con parámetros explícitos y ejecutarla con Cargo. Las pruebas de integración de la raíz usan el nombre de archivo sin extensión como argumento de `cargo test --test`. Los módulos enlazados incluyen los contratos de los métodos públicos. Los diagnósticos de la auditoría están en [el registro](DOC_AUDIT.md).

| Capacidad y propietario | API y uso | Prueba o recorrido | Alcance o contrato canónico |
| --- | --- | --- | --- |
| [air](../src/air.rs) | `Air`: c, ρ y referencia de presión para todos los solvers | [acoustic_consistency](../tests/acoustic_consistency.rs) | Medio homogéneo |
| [driver](../src/driver.rs) | `Driver, CoilModel`: parámetros electromecánicos → impedancia bloqueada, movimiento y T/S | [coil_model](../tests/coil_model.rs) | Mmd y carga de aire se contabilizan por separado |
| [driver_bank](../src/driver_bank.rs) | `DriverBank::equivalent_driver`: drivers idénticos y cableado → driver equivalente | [driver_bank](../tests/driver_bank.rs) | Sin separación espacial |
| [driver_import](../src/driver_import.rs) | `import_driver`: texto WDR/OWDR → driver y metadatos | [driver_import](../tests/driver_import.rs) | Conversión de Mms a Mmd según formato |
| [segment](../src/segment.rs) | `Segment, Horn, Flare`: área axial → matriz y carga de garganta | [completion_numerics](../tests/completion_numerics.rs) | Propagación unidimensional |
| [horn_profile](../src/horn_profile.rs) | `HornProfileSpec`: parámetros SI → radio, muestras, volumen y segmento | [horn_profile](../tests/horn_profile.rs) | Geometría comprobada independientemente del campo |
| [thermoviscous](../src/thermoviscous.rs) | `ThermoviscousAir`: capa límite delgada → propagación compleja | [horn_simulation_reference](../tests/horn_simulation_reference.rs) | Consultar dominio del tubo circular |
| [radiation](../src/radiation.rs) | `piston_radiation_impedance`: área y aire → carga de pistón | [acoustic_consistency](../tests/acoustic_consistency.rs) | Pantalla infinita |
| [element](../src/element.rs) | `Chamber, Port, PassiveRadiator`: volumen, conducto o membrana → carga local | [system_production](../tests/system_production.rs) | La corrección de extremo depende de la terminación |
| [system](../src/system.rs) | `System, FrontAdapter`: caja, puerto, PR o bocina → respuesta/energía | [system_production](../tests/system_production.rs) | Adaptador opcional con parámetros explícitos |
| [tapped](../src/tapped.rs) | `TappedHorn`: dos tomas de un diafragma → estado TH o TH1 | [th_production](../tests/th_production.rs) | Comprobar posición exacta de ambas tomas |
| [offset](../src/offset.rs) | `OffsetHorn`: toma desplazada y camino trasero → suma compleja | [od_production](../tests/od_production.rs) | Presión combinada |
| [bandpass](../src/bandpass.rs) | `Bandpass`: cámaras y puertos → estado BP4, BP6P o BP6S | [bp_production](../tests/bp_production.rs) | El puerto interno de BP6S no radia |
| [transmission_line](../src/transmission_line.rs) | `TransmissionLine`: tramos, toma y frontera → presión y caudal | [transmission_line](../tests/transmission_line.rs) | Experimental, sin radiación mutua |
| [network](../src/network.rs) | `AcousticNetwork, NetworkLayout`: grafo → solución compleja diferencial | [acoustic_network](../tests/acoustic_network.rs) | Equivalencia por variable, topologías arriba |
| [design](../src/design.rs) | `Design, DesignPoint`: topología clásica → punto o barrido preparado | [prepared_engine](../tests/prepared_engine.rs) | Punto resumen con campos legados |
| [phase](../src/phase.rs) | `phase_trace`: fase muestreada → fase continua y retardo opcional | [energy_regressions](../tests/energy_regressions.rs) | Una cancelación interrumpe la derivada |
| [energy](../src/energy.rs) | `EnergyReport`: estados RMS → potencia activa, pérdidas y calidad radiativa | [energy_physics](../tests/energy_physics.rs) | Separar fuente, bornes y cargas |
| [maximum_spl](../src/maximum_spl.rs) | `UnitDriveResponse`: 1 V RMS → máxima salida lineal por frecuencia | [maximum_spl](../tests/maximum_spl.rs) | Requiere límite vinculante y observación disponible |
| [multiway](../src/multiway.rs) | `sum_at, sum_sweep`: vías con polaridad y retardo → suma de presiones | [multiway_oracles](../tests/multiway_oracles.rs) | Fuentes compactas, misma distancia |
| [calibration](../src/calibration.rs) | `Calibration::validated_60_70`: factores empíricos → observación calibrada | [production_accuracy](../tests/production_accuracy.rs) | Válidos para geometrías comparadas |
| [record](../src/record.rs) | `LegacyRecord, HornrespTextRecord`: bytes o plantilla → parámetros reproducibles | [hornresp_text_roundtrip](../tests/hornresp_text_roundtrip.rs) | El intercambio textual admite un subconjunto |
| [measured_data](../src/measured_data.rs) | `Measurement`: FRD/ZMA y referencia → muestras/interpolación | [measured_data](../tests/measured_data.rs) | Conservar fase ausente y procedencia |
| [reference_design](../src/reference_design.rs) | `sealed_alignment, uniform_transmission_line`: propuesta inicial → parámetros de diseño | [reference_integration](../tests/reference_integration.rs) | Alineamiento ideal y aproximación de TL |
| [alignments](../src/alignments.rs) | `sealed, vented, extended_bass_shelf`: familia y T/S → diseño inicial y desviación | [alignments](../tests/alignments.rs) | Contrastar con el solver cargado |
| [optimization](../src/optimization.rs) | `minimize, screen_and_refine`: variables y objetivos → candidatos y causas de rechazo | [horn_fidelity](../tests/horn_fidelity.rs) | El presupuesto limita la cobertura |
| [validation](../src/validation.rs) | `OraclePoint, validate`: export → errores por variable y recuentos | [production_accuracy](../tests/production_accuracy.rs) | La fase eléctrica requiere un diagnóstico aparte |
| [error](../src/error.rs) | `SolveError`: distinguir entrada inválida de fallo numérico | [engine](../tests/engine.rs) | Propagar errores de métodos estrictos |

## Dependencias y conversiones

La única dependencia entre bibliotecas es `speakerbox-measure → speakerbox-circuits`. La raíz declara esa crate solo como dependencia de desarrollo del [ejemplo ejecutable](../examples/crossover_end_to_end.rs); ninguna biblioteca publicada depende de otra. La aplicación compone las tres. La raíz usa su propia `measured_data::Measurement` con referencia y fase opcional. `speakerbox-circuits::ComplexCurve` es el tipo complejo utilizado por circuitos y por `ParsedCurve` de measure. Para convertir, alinee frecuencias, unidades, fase y referencia de amplitud antes de construir otra curva. Un FRD en dB necesita una referencia de presión para interpretarse en Pa.

| Crate y módulos | Entrada → resultado | Recorrido verificable y límites |
| --- | --- | --- |
| [circuits](../crates/speakerbox-circuits/src/): model, solver, analysis, curves, error | Fuente ideal, tierra, R, L con resistencia serie, C y carga compleja → MNA, tensión, impedancia de entrada y transferencia | [RLC analítico](../crates/speakerbox-circuits/tests/analytic.rs). La red necesita una referencia y una solución única. |
| circuits: crossover, equalizer, optimize, cir | Butterworth o Linkwitz–Riley ideales → transferencia. Escalera pasiva Butterworth de órdenes 1–4, L-pad y Zobel → componentes. Optimización de valores contra magnitud objetivo. Lectura/escritura del subconjunto .CIR | [Crossover](../crates/speakerbox-circuits/tests/crossover.rs), [netlist](../crates/speakerbox-circuits/tests/cir_fixture.rs). Pico y shelves se componen como transferencias analógicas ideales. |
| [measure](../crates/speakerbox-measure/src/): curve_file, equiv, fit, error | FRD/ZMA → curva compleja. ZMA → semilla → ajuste Levenberg–Marquardt positivo → siete parámetros, residuos y estado de convergencia | [Identificación](../crates/speakerbox-measure/tests/identification.rs). Examinar fase, magnitud, cobertura de resonancia y advertencias. Un coste bajo admite parámetros correlacionados. |
| measure: experiments, validate | Masa añadida conocida y caída de Fs → Mms y Cms. Caja sellada conocida y aumento de Fs → Vas. Curvas separadas y combinada con metadatos → comparación multivía | [Experimentos](../crates/speakerbox-measure/src/experiments.rs), [dataset sintético](../crates/speakerbox-measure/tests/validation_dataset.rs). Hay que medir o aportar Sd y Bl para construir un driver. |

**Circuito y carga acústica.** Evalúe el recinto para obtener la impedancia en bornes del driver. En el resumen de `System`, retire Rg de la impedancia compleja si se incluyó, o use directamente la impedancia de bornes del estado que la expone. Úsela como carga del circuito a las mismas frecuencias. La solución MNA da tensión compleja en la carga. Si usa esa tensión como excitación acústica, ajuste la referencia de generador para evitar descontar dos veces la impedancia de salida. La presión requiere además el modelo de recinto y radiación. El recorrido ejecutable de esta cadena está en [`examples/crossover_end_to_end.rs`](../examples/crossover_end_to_end.rs).

**Optimización.** Declare límites y restricciones físicas antes del coste. Evalúe candidatos viables mediante el solver elegido. Conserve semilla, presupuestos, cancelación, configuración y rechazos. El refinamiento vuelve a evaluar candidatos con el backend suministrado por la aplicación. Compare los resultados por frecuencia y valide el finalista con evidencia independiente. [Ensayo de cribado](../tests/horn_fidelity.rs).
