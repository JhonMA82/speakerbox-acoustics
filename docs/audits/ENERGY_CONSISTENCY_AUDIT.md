# Segunda auditoría: energía y radiación

Base real: `5e4388363fa248f49abd2e4a164949dbfdaf7b0b`, rama
`fix/acoustic-physical-consistency`, árbol limpio. Nueva rama local:
`fix/acoustic-energy-consistency`. Se preservan todos los commits de la primera
auditoría; main no se mueve.

Se releen contratos y los cinco informes anteriores. Línea base recién
ejecutada, no tomada de esos informes: suite debug/release, producción,
comparación ampliada y cinco benchmarks. Registros en `energy-raw/base-*`.
Se confirman matriz exacta de área constante, matrices locales y PreparedDesign.

EC-001 confirmado: Horn resuelve una carga distinta de la utilizada por
System para reportar potencia. Matched_mouth tampoco define una función de
transferencia a un punto del campo lejano.

EC-002 confirmado: las potencias se deducían del campo estimado o multiplicaban
el flujo de boca por 2π/Ω. Ese escalado no pertenece a la carga de la red.
Las regresiones anteriores a cambios están en `energy-raw/regressions-before.log`.
No se cambia la calibración ni un archivo de referencia.

## Derivación e implementación final

### EC-001 — Crítica, confirmada y corregida

Horn::termination_impedance centraliza la carga que usa Horn::prepare.
System reporta |Um|²Re(Zterm), no la resistencia de un pistón de otra área,
ni un factor de campo. Matched usa ρc/Sgeom, y es incompatible con factores
que no se cancelen. Con factores extremos iguales se evita formar productos
intermedios infinitos. P=Re(pU*) se comprueba contra |U|²Re(Zterm), además de
comprobar ausencia de onda reflejada en un cilindro adaptado.

Un absorbente no determina su campo exterior: try_energy_at está disponible,
pero try_response_at da error explícito por SPL no disponible y la observación
tipada devuelve None. No se simula un segundo pistón.

### EC-002 — Crítica, confirmada y corregida

Los puntos de todas las topologías reportan ahora flujo de cargas realmente
incluidas. La conversión de presión de campo a potencia y 2π/Ω se eliminan
de los valores energéticos (sin cambiar SPL calibrado).

Balance eléctrico:

`Psource = PRg + Pcoil + Pmech + Pacoustic_input`.

Balance acústico:

`Pacoustic_input = Pwall + Plining + PRseries + Pleak + PQL + Ppr_mech + ΣPterm`.

Cada término procede de un elemento, no de una resta de energía desconocida:

- Serie: R|U|². Fuga/resistencia de QL: |p|²/R.
- Revestimiento: |p|²Re(Ylining).
- PR: Rms|Upr/Sd|². No se lo llama potencia radiada.
- Webster con conductancia g=loss·ωS/(ρc²): de dRe(pU*)/dx=-g|p|² se integra
  Pwall=∫g|p|²dx con las etapas RK4 del estado real. La pérdida no se define
  como diferencia de flujos; esa diferencia queda como contraste independiente.
- Los puertos Pipe son sin pérdidas: potencia activa de entrada igual a la
  carga exterior. BP reutiliza las impedancias ya calculadas para evitar
  repetir Bessel/Struve. El puerto interior BP6 serie no se suma como radiador.

EnergyReport distingue potencia de fuente, bornes, pérdidas y terminaciones.
El residual es diagnóstico; no cierra artificialmente el balance. Los métodos
científicos de eficiencia devuelven Option y conservan la calidad radiativa.

### EC-003 — Alta, coherencia de cargas/campo

El flujo de una carga se separa de la presión en un punto. Las contribuciones
al campo legado no se modifican salvo una omisión confirmada: OD con tap=0
sin cámara tenía carga trasera pero omitía su presión. Ahora también radia
su cara posterior bajo la misma aproximación compacta de las otras tomas.

La falta de geometría espacial no se corrige inventando distancias. Las cargas
independientes, las calibradas y los modelos sin alguna carga exterior se
etiquetan de forma diferente; ver RADIATION_MODEL_CONTRACT.md. No se modifican
factores de calibración ni se añaden resistencias radiativas arbitrarias.

### EC-004 — Alta, fase y retardo

La primera auditoría ya corrigió la fase total OD, orden/duplicados y derivada
centrada. Se conserva esa solución. Se confirma un hueco: arg(0) se presentaba
como fase 0°. Ahora una presión nula da NaN en el punto legado y None en la
observación tipada. La eficiencia con excitación cero también es indefinida.
AcousticObservation asocia presión, SPL, fase y retardo de la misma magnitud.
complex_phase_trace admite cotas de redondeo en cancelaciones profundas.

### EC-005 / EC-006 — Evidencia y contrato

Se añaden pruebas analíticas de adaptación, resistencia pura, pérdidas
identificadas, oscilador, circuito eléctrico, Helmholtz y reciprocidad.
Hay convergencia por subdivisión geométrica de un tramo con pérdidas, rejillas
lineales/logarítmicas y fase de un oscilador cerca de resonancia.
Las APIs nuevas son aditivas; no se añadió una arquitectura espacial.

## Descartes y límites demostrados

- No es correcto cambiar automáticamente la impedancia por Ω: no existe una
  geometría de imágenes definida. Ω conserva su papel de presentación.
- Un residual de Hornresp grande no determina qué resistencia física añadir.
  Se corrigen identidades de flujo sin retocar coeficientes.
- No es correcto sumar presiones para obtener potencia integral ni anular
  toda potencia porque dos puertos cancelen en un observador compacto.
- PR Rms y pérdidas QL no son cargas radiativas. No se reclasifican para
  reducir el error de potencia.
- Masa añadida, compliancias e inductancias ideales no disipan en régimen
  estacionario; se conservan los modelos de masa anteriores.

Sigue sin implementarse acoplamiento mutuo, mapa espacial, resistencia
radiativa faltante en puertos concentrados/System PR ni un modelo de Ω exacto.
Su resultado radiativo total queda no disponible o aproximado, no falsamente
validado. Las limitaciones históricas numéricas de RK4 y del solver nodal
fuera de su dominio permanecen; el informe expone su residual de balance.

## Ubicación y trazabilidad

Core: src/energy.rs; carga: src/segment.rs; ensamblado y desglose en
system/tapped/offset/bandpass::try_energy_at; fachada y observación en
design.rs; fase con incertidumbre en phase.rs.
Commit de implementación: `11f9d6c`. Los logs y los otros tres informes
contienen comandos, métricas y validación final.
