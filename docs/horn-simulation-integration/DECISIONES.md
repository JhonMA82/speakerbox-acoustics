# Transferencia y decisiones científicas

Se comparó `speakerbox-acoustics` en `dbcd8eeba183b1a8af7722e706c2bfa234206efc`
con `horn-simulation` en `7dd9852aec8f60e1b35ae637ad28bd614efc2c1b`.
Los PR #6 a #9 del repositorio principal habían añadido suma multivía,
oráculos y un validador de datasets. En la referencia ya existían la corrección
del alias `lecleach` y la separación entre TMM de cribado y FEM. La auditoría
comprobó `segment`, `radiation`, `driver`, `system`, `transmission_line`,
`optimization`, `energy`, oráculos y los informes previos. En la referencia
se examinaron `profiles`, `webster`, `duct`, `modal_radiation`,
`driver_interface`, `solver`, `meshing`, `prescreen`, `lem_prescreen`,
`rank_pipeline`, `refine` y los contratos de física y validación.

| Capacidad | Clasificación | Decisión |
| --- | --- | --- |
| Webster con perfiles básicos, RK4 y solución uniforme | Ya implementada correctamente | Reutilizar `Segment` y `Horn`. Ejecutar el TMM Python como comparación externa. |
| Compliancia delantera, resorte trasero, transformación por Sd² y bobina Le/Ke | Ya implementada correctamente | Conservar motor, masas y la convención existente de potencia activa RMS. |
| Carga de pistón y absorbente adaptado | Ya implementada correctamente | Comparar casos con condiciones idénticas. `radiation.rs` incorpora el término reactivo y el ángulo de observación sigue separado de esa carga. |
| Pérdidas Darcy, conductancia de pared y recubrimientos | Ya implementada correctamente | Conservar; representan mecanismos distintos de la capa límite termoviscosa. |
| Tractrix, tractrix escalada, OS geométrica y exponencial-cónica | Nueva capacidad viable para integración inmediata | Proporcionar parámetros serializables y un único perfil para muestras y solver. |
| Cámara cilíndrica uniforme con área y volumen especificados | Nueva capacidad viable para integración inmediata | Abrir un tramo distribuido explícito, con inercia y retardo de propagación. |
| Hypex para registros sin T | Implementada, pero mejorable | Corregir su nombre y atribución. Mantener `1/√2` y el dominio del solver. |
| Filtro, diversidad y presupuesto por fidelidad | Implementada, pero mejorable | Mantener Nelder–Mead y DE. Añadir cribado/refinado de un conjunto de candidatos, con costes y rechazos por etapa. |
| Evidencia y comparación de fase, señal compleja y potencia | Implementada, pero mejorable | Añadir una tabla de alcance y un manifiesto comprobado, usando también oráculos existentes. |
| Capa límite termoviscosa delgada en tubo circular | Nueva capacidad viable para integración inmediata | Implementación optativa, con diagnóstico fuera de dominio y comparación con Kirchhoff. La evidencia física por medición sigue pendiente. |
| Kirchhoff completo en perfiles variables, paredes no circulares y singularidades de borde | Capacidad experimental que requiere validación | Conservar como referencia analítica externa para el tubo y diferir la extensión. |
| Radiación modal y Helmholtz 3D | Capacidad experimental que requiere validación | Mantener un contrato externo. Una interfaz de 1D no puede representar todas las variables de apertura. |
| JMLC auténtico | Capacidad experimental que requiere validación | Construcción independiente pendiente de coordenadas de referencia y pruebas de convergencia. |
| Estimar volumen frontal en función de Sd y usar Mms como masa mecánica en una red con carga explícita | No recomendable para integración | Los supuestos son dependientes de la geometría y pueden contar dos veces masa acústica. |
| Incorporar Nextflow, contenedores, DOLFINx y Gmsh a la crate | No recomendable para integración | Dejar el ciclo FEM fuera del motor Rust y de WASM. |
| Ajustar calibración Hornresp a la comparación TMM o mezclar propagación esférica y Webster plano en el mismo oráculo | No recomendable para integración | Cada cálculo conserva sus condiciones de frontera, variables y criterio. |

## Hypex, tractrix y Le Cléac’h

1. **Salmon Hypex.** El radio plano satisface `r'' = r/xT²` para todo el
   tramo, con `r'(0) = T·r(0)/xT`. En consecuencia `T = 1/√2` fija la
   pendiente inicial dentro de la misma familia. Se mantiene en el importador
   `Hyp` como valor histórico. [Salmon, JASA 17(3), 212–218, 1946](https://doi.org/10.1121/1.1916317).
2. **Tractrix de tangente constante.** Para `a` constante y `0 < r ≤ a`,
   `dx/dr = √(a²−r²)/r`. De ahí sale el parámetro
   `r=a·sech(t)`, `x=a·[(t_t−tanh(t_t))−(t−tanh(t))]`.
   Con `a=r_m` la boca acaba perpendicular al eje. Para una longitud mayor
   se ajusta `a>r_m`. La pendiente de pared diverge al final de la tractrix
   completa, en contraste con la ley Hypex finita. [Kolbrek, Horn Theory](https://www.grc.com/acoustics/An-Introduction-to-Horn-Theory.pdf).
3. **Tractrix escalada.** Se multiplica la coordenada axial de la curva
   completa por `L/L_nat`. Para una escala `λ`, la longitud del segmento
   tangente al eje pasa a `√(r²+λ²(a²−r²))`. Depende de `r` salvo
   `λ=1`. El alias `lecleach` de la referencia apunta a esta variante,
   que en Python se interpola sobre 500 puntos. Rust resuelve el contorno
   continuo sin esa interpolación.
4. **JMLC.** La construcción de Jean-Michel Le Cléac’h hace evolucionar la
   forma del frente por avances normales en pasos pequeños, con pared
   ortogonal al frente. La ley de Salmon prescribe en esa construcción el
   área **curva del frente de onda**. Aplicarla al disco plano a lo largo
   del eje da otra geometría. Un futuro JMLC debe contar con frente inicial,
   criterio de avance, truncado y un oráculo geométrico independiente.
   [Le Cléac’h, ETF 2010](https://pearl-hifi.com/06_Lit_Archive/14_Books_Tech_Papers/LeCleac%27h_Jean-Michel/Horn_Profiles.pdf) · [Kolbrek, figura 24](https://www.grc.com/acoustics/An-Introduction-to-Horn-Theory.pdf).

La guía OS usa `r² = r_t² + x²·tan²(θ)`. El modo 1D calcula Webster
sobre el área de ese contorno. El modelo de propagación en coordenadas
esferoidales de Geddes tiene otro alcance. [Geddes y Lee, AES](https://aes.org/publications/elibrary-page/?id=6998).

## Acoplamiento del motor

En ambos modelos `U_d = Sd·v_d`, `Z_a=p/U` se refleja al motor como
`Sd²·Z_a` y la cámara trasera ideal aporta `ρc²/(jωV)`. El TMM de
referencia informa `z=p/v` específico. Para obtener `Z_a` se divide por
el área física de entrada. En Rust, `Driver::mechanical_impedance` parte
de Mmd. Mms incluye masa acústica en la caracterización al aire libre.
`horn_core.parameters` usa Mmd más carga trasera si ambas están caracterizadas
y Mms en caso contrario. La referencia presupone `Re+jωLe`; Rust conserva
además la opción de semiinductancia y sus pérdidas. Se igualó Mmd sintética
con la masa efectiva del motor Python para comprobar la misma red, sin valores
de catálogo ni carga trasera adicional.

Un volumen concentrado deriva caudal sin introducir retardo de propagación.
Para el adaptador uniforme `L=V/A`, la expansión de su matriz a baja
frecuencia añade compliancia `V/(ρc²)` y masa acústica `ρL/A`. El límite
`V=0` conserva exactamente la conexión ideal. Usar un área uniforme
equivalente es un supuesto geométrico que debe declarar quien provee los
datos. El caso de prueba tiene área y volumen sintéticos fijados.

## Pérdidas de capa límite

Con `e^(+jωt)`, las profundidades son
`δv = √(2μ/(ρω))` y `δt = √(2κ/(ρ·Cp·ω))`. En radio `a`,
`Fv=(1−j)δv/a` y `Ft=(1−j)δt/a`. La resistencia serie y la
admitancia en paralelo por unidad de longitud provienen de
`ρ_eff=ρ/(1−Fv)` y `C_eff=[1+(γ−1)Ft]/(ρc²)`:

```text
Z' = jω ρ_eff / S
Y' = jω C_eff S
dP/dx = −Re(Z') |U|² − Re(Y') |p|²
```

La parte real de ambos coeficientes es positiva en el dominio admitido. La
matriz del cilindro es exacta para esos coeficientes y el balance integra
las dos pérdidas sobre estados propagados. La restricción
`max(δv,δt)/a ≤ 0.1` delimita la aproximación delgada. La restricción
`ka ≤ 1.5` deja margen por debajo de la propagación de modos transversales.
La composición puede alternar secciones termoviscosas con secciones Darcy,
pared o revestimiento, con una sola ley distribuida en cada sección.
[Berggren, Bernland y Noreland, 2018](https://doi.org/10.1016/j.jcp.2018.06.005) · [Stinson, JASA 89, 550–558, 1991](https://doi.org/10.1121/1.400379).

## Licencias y procedencia

La revisión Python tiene licencia MIT de Tim Richardson en la raíz y archivos
MIT de paquete en core, geometry, solver y analysis. Rust conserva MIT OR
Apache-2.0. Las ecuaciones se implementaron en Rust a partir de su
formulación matemática y usando el solver existente. El fixture se ejecutó
sobre un checkout limpio de MIT con geometrías, excitación y motor sintéticos.
Se revisaron los registros de fuentes externos. Hajicek/Neely y driver-base
declaran CC BY 4.0, Openwind GPL-3.0 y bempp-cl MIT. Varias mediciones de tubos
permiten descarga pero carecen de un permiso explícito de redistribución.
Los datos de medición, los oráculos ajenos y sus archivos quedan en sus
repositorios originales. [MIT de horn-simulation](https://github.com/timini/horn-simulation/blob/7dd9852aec8f60e1b35ae637ad28bd614efc2c1b/LICENSE) · [Catálogo de validación](https://github.com/timini/horn-simulation/blob/7dd9852aec8f60e1b35ae637ad28bd614efc2c1b/data/validation/references.json).
