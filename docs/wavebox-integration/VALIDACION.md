# Validación de redes y topologías

## Equivalencia con los solvers existentes

| Ruta | Alineación física y observables |
| --- | --- |
| Caja ventilada | Misma cámara concentrada, inertancia y condición de presión libre a la salida. Ze, Xd, caudal de puerto, SPL y fase del observador legado y potencia activa coinciden dentro de 10⁻¹⁰ relativo. El test añade el mismo observador externo de puerto a ambos resultados. La potencia de esa salida ideal es cero por la condición p=0. |
| TH | Misma abertura a 2π y misma masa mecánica efectiva. La prueba de red añade a Mmd la masa que el solver TH histórico incluye explícitamente. Ze, Xd, caudal de boca, SPL y potencia acústica coinciden dentro de 10⁻⁶ relativo. Las calibraciones de producción se conservan. |
| BP4 | Misma cámara concentrada, tubo y pistón. Ze, caudal complejo de diafragma y puerto, excursión y potencia coinciden dentro de 2·10⁻¹⁰ relativo. SPL y fase coinciden aplicando a ambos el observador compacto del BP4 existente. |
| Conducto ramificado | Referencia analítica con impedancias de dos entradas en paralelo. Reciprocidad, presión, continuidad de flujo, extremo cerrado, potencia y permutación de nodos dentro de 10⁻¹¹ relativo. Se prueba el caso B≈0 a media onda, una carga de 10⁹ Pa·s/m³ y fallos estructurales o numéricos. |

## Comparación con Hornresp

Se fijó Wavebox `4195c93f757a99c4bb2af456ff8dd8421a948510` y
371 muestras por caso de 10 a 2,000 Hz, aire 344 m/s, 1.205 kg/m³,
2.83 V RMS y 2π. Los CSV contienen las columnas de los archivos de diseño
de Wizard y los dos exports All Chart Data de driver avanzado. SPL usa
20 µPa. WPhase se compara con la presión física de la red, y ZePhase
con la impedancia en bornes. Las fases se comparan circularmente. Para
WPhase se exige referencia SPL ≥ 50 dB. La métrica Xd descuenta 0.0005 mm
por redondeo y divide por `max(Xd_ref, 0.05 mm)`. El error Ze es complejo
relativo y no un error de magnitud solamente. El caudal PH1 procede del
solver independiente de Wavebox, porque los archivos del Wizard carecen de
columnas de caudal.

| Caso | SPL RMS / máx dB | Ze complejo máx | Ze fase máx | Xd normalizado máx | WPhase máx | Estado |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| PH1 | 0.0166 / 0.147 | 0.707 % | 0.388° | 1.151 % | 0.782° | Pasa fixture. Caudal complejo máx 0.049 %. |
| BP8S | 0.0069 / 0.050 | 0.688 % | 0.351° | 1.148 % | 0.482° | Pasa con la fuga explícita del fixture. |
| DBR | 0.0193 / 0.242 | 0.524 % | 0.270° | 2.808 % | 1.120° | Pasa con la fuga explícita del fixture. |
| BPB | 0.0247 / 0.128 | 1.181 % | 0.575° | 4.014 % | 0.566° | Experimental. Falla la excursión puntual. |
| ABC | 0.4239 / 2.789 | 18.243 % | 8.811° | 120.79 % | 8.583° | Diagnóstico. Falla la comparación puntual. |
| BPC | 8.1194 / 19.003 | 1.932 % | 0.789° | 538.95 % | 179.93° | Diagnóstico. Falla la observación acústica y la excursión. |
| Semiinductancia extendida | 0.0044 / 0.0113 | 0.025 % | 0.0141° | Dentro del redondeo | 0.227° | Pasa recinto sellado del fixture. |
| FDD | 0.0044 / 0.0113 | 0.030 % | 0.0170° | Dentro del redondeo | 0.227° | Pasa recinto sellado del fixture. |

Gates de nuevas topologías: SPL RMS < 0.5 dB y máximo < 1.5 dB,
Ze complejo < 3 %, fase de Ze < 2°, Xd < 3 %, WPhase < 3° y balance
relativo < 2·10⁻⁶ respecto a potencia de entrada. PH1 contrasta también
caudal (umbral 2 %) y subdivisión del cono más asimétrico. La prueba
conserva la discrepancia de BPB y los diagnósticos ABC/BPC. También
conserva el DBR sin fuga, que alcanza 1.409 dB RMS y 25.99 dB de error
máximo. El balance energético calculado por la red en PH1 está dentro
de 9.69·10⁻⁹ relativo. Para BP8S, BPB y DBR, dentro de 8.0·10⁻¹⁶.

Los coeficientes de corrección de extremo y la conductancia de fuga
usados por Wavebox están en el montaje del fixture. El resultado del
fixture no fija una fórmula QL general para estos recintos. Para BPC se
comprobó el uso de C1–P1–C3–P3 y la segunda salida C2–P2. La coincidencia
de Ze convive con discrepancias de SPL, fase y Xd. Hace falta contrastar
caudales complejos por puerto, referencia del lado del diafragma y
convención de suma con datos adicionales antes de seleccionar otro modelo.
ABC añade una salida directa y dos ramas del lado trasero. Su ajuste
publicado de SPL RMS deja errores puntuales mayores en Ze y Xd. La
variante paralela BP8P se prueba en simulación con balance, a la espera
de curvas independientes.

## Procedencia y reproducción

El código y los archivos de Wavebox se consultaron bajo MIT. Se conserva
el aviso de copyright junto a las series numéricas normalizadas. Hornresp
es el programa externo, con distribución propia. Los fixtures sólo
contienen números y parámetros de montaje reproducibles. No se realizaron
exportaciones propias ni mediciones en esta sesión. El entorno carecía
de Hornresp y Wine. [Wavebox, licencia](https://github.com/Padwaywaydonc/wavebox/blob/4195c93f757a99c4bb2af456ff8dd8421a948510/LICENSE) · [Decisiones de Wavebox](https://github.com/Padwaywaydonc/wavebox/blob/4195c93f757a99c4bb2af456ff8dd8421a948510/README.md).

```bash
cargo test --locked --test acoustic_network --test wavebox_hornresp -- --nocapture
python tools/cross-validation/wavebox/generate.py ../wavebox /tmp/wavebox-fixtures
diff -u tests/data/wavebox/ph1a.csv /tmp/wavebox-fixtures/ph1a.csv
python tools/cross-validation/wavebox/inspect_pore.py ../wavebox
```

El generador exige la revisión fijada y NumPy/SciPy (entorno verificado
NumPy 2.3.5 y SciPy 1.17.0). Para capturar series nuevas con una
instalación externa, los scripts Wavebox pueden exportar curvas del Wizard
abierto con la topología configurada. Ejecútelos en una instalación de
prueba aislada. `run_record.sh` limpia el directorio Import de esa
instalación. Para PH1 documente ambas vías en el registro o en un
manifiesto, ya que el formateador Python de la referencia no emite
las áreas y longitudes de la vía superior. [Scripts externos](https://github.com/Padwaywaydonc/wavebox/tree/4195c93f757a99c4bb2af456ff8dd8421a948510/tools/hornresp).
