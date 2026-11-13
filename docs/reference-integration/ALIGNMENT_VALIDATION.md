# M7 — síntesis verificable

Implementación independiente `alignments`, sin fórmulas orientativas copiadas.

Selladas: Butterworth Q=1/√2; Bessel Q=1/√3; Chebyshev especifica pico en dB
respecto al límite de alta frecuencia, resolviendo Q⁴/(Q²−1/4)=10^(ripple/10).
Volumen y F3 reutilizan la síntesis exacta previa; se rechaza Qtc≤Qts.

Ventiladas ideales (Le=0, sin pérdidas, Mms constante): s normalizado a ωs,
H=s⁴/[s⁴+s³/Qts+(1+α+h²)s²+h²s/Qts+h²], h=Fb/Fs, α=Vas/Vb.
Para prototipo s⁴+b3 s³+b2 s²+b1 s+b0, la escala r²=b1/(b3 b0),
h²=b0 r⁴, Qts=1/(b3 r), α=b2 r²−1−h². No se cambia Qts del driver:
B4, Bessel4 o C4 con ripple específico rechazan incompatibilidad >1e-7.
B4 da Qts=sin(π/8), α=√2, h=1; no Vb=Vas universal.
Bessel4 se obtiene invirtiendo el polinomio Bessel [1,10,45,105,105].
C4 y SC4 multiplican polos conjugados −k sinθ+j cosθ, θ=(2i+1)π/8.
C4 k=tanh(asinh(1/ε)/4); SC4 resuelve k≥1 para Qts del driver.
SC4 se limita al dominio realizable de esta familia de polos conjugados,
con error explícito fuera de él (no pretende cubrir variantes con polos reales).
QB3 impone nulos los términos ω⁶ y ω⁴ de |D(jω)|², dando
h²=1/[8 Qts²(1−Qts²)], α=1/(2Qts²)−1−h²; Qts≤sin(π/8).
F3 se localiza por raíz del módulo, no mediante múltiplos empíricos de Fb.

EBS no tiene solución única: `extended_bass_shelf` optimiza con el solver
completo para profundidad, frecuencia de esquina, límites de volumen/sintonía
y área de puerto explícitos. Devuelve mejor propuesta y coste/convergencia;
no garantiza una plantilla exacta ni oculta error. En el caso sintético de
prueba, coste final=15.58132473 dB² (~3.9473 dB RMS): propuesta orientativa,
NO alineamiento exacto ni respuesta validada con esa precisión. El consumidor
debe rechazar propuestas cuyo coste exceda su tolerancia.

`to_system` produce una geometría válida usando Helmholtz y corrección de
extremos suministrada; rechaza longitud negativa en lugar de fijarla a un mínimo.
`solver_deviation_db` mide separación del ideal frente al solver completo.
B4 sintético (pistón pequeño, Le=0), f/Fs=0.5/1/2/3: errores observados
+0.00017210, −0.00000243, −0.00085658, −0.00212043 dB. Son aproximaciones
esperadas de radiación frente a T/S ideal, no nuevas tolerancias Hornresp.
Pruebas también verifican las identidades de coeficientes QB3, α Bessel=7/3,
pico C4=1 dB y F3 exacta de las familias selladas. 3/3 pruebas pasan.

Fuentes consultadas para contraste de ecuaciones (no código): Small,
Vented-Box Loudspeaker Systems, 1973; derivación de A. C. en
https://www.andyc.diy-audio-engineering.org/vented-box/vented_box_2.html
(QB3: coeficientes de |D|²; C4/SC4: síntesis por polos).
