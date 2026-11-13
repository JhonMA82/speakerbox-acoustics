//! Segmentos de guía de onda y ensamblado de la bocina.
//!
//! Cada segmento se describe por su ley de área `S(x)` y su longitud. El motor
//! integra la ecuación de Webster
//!
//! ```text
//! d/dx ( S(x)·dp/dx ) + k²·S(x)·p = 0
//! ```
//!
//! escrita como sistema de estado acústico:
//!
//! ```text
//! dp/dx = −j·ω·ρ·U / S(x)
//! dU/dx = −j·ω·S(x)·p / (ρ·c²)
//! ```
//!
//! con `p` presión acústica (Pa) y `U` velocidad volumétrica (m³/s). La
//! integración RK4 produce la matriz de transferencia
//! `[p2; U2] = M·[p1; U1]`, válida para cualquier perfil (cónico, exponencial,
//! parabólico, hiperbólico-exponencial o definido por el usuario).
//!
//! Los perfiles analíticos del software de referencia (con/exp/par/hyp) coinciden con las
//! matrices exactas de Webster; la integración numérica converge a ellas con
//! un paso mucho menor que la longitud de onda.

use std::fmt;
use std::sync::Arc;

use num_complex::Complex64;

use crate::air::Air;
use crate::omega;
use crate::radiation::piston_radiation_impedance;

/// Matriz de transferencia 2×2 para el vector de estado `[p, U]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransferMatrix {
    pub a: Complex64,
    pub b: Complex64,
    pub c: Complex64,
    pub d: Complex64,
}

impl TransferMatrix {
    /// Matriz identidad (segmento de longitud nula).
    pub fn identity() -> Self {
        Self {
            a: Complex64::new(1.0, 0.0),
            b: Complex64::new(0.0, 0.0),
            c: Complex64::new(0.0, 0.0),
            d: Complex64::new(1.0, 0.0),
        }
    }

    fn checked(self) -> Result<Self, crate::SolveError> {
        if [self.a, self.b, self.c, self.d]
            .iter()
            .all(|z| z.re.is_finite() && z.im.is_finite())
        {
            Ok(self)
        } else {
            Err(crate::SolveError::InvalidResponse(
                "nonfinite transfer matrix",
            ))
        }
    }

    /// Aplica la matriz a un estado `[p, U]`.
    pub fn apply(&self, p: Complex64, u: Complex64) -> (Complex64, Complex64) {
        (self.a * p + self.b * u, self.c * p + self.d * u)
    }

    /// Cascada geométrica: aplica primero self y después next.
    pub fn then(self, next: Self) -> Self {
        Self {
            a: next.a * self.a + next.b * self.c,
            b: next.a * self.b + next.b * self.d,
            c: next.c * self.a + next.d * self.c,
            d: next.c * self.b + next.d * self.d,
        }
    }

    /// Transforma una impedancia `Z = p/U` del plano de salida al de entrada.
    pub fn transform_impedance(&self, z_out: Complex64) -> Complex64 {
        if !z_out.re.is_nan()
            && !z_out.im.is_nan()
            && (z_out.re.is_infinite() || z_out.im.is_infinite())
        {
            if self.c == Complex64::new(0.0, 0.0) {
                return Complex64::new(f64::INFINITY, 0.0);
            }
            return -self.d / self.c;
        }
        let num = self.b - self.d * z_out;
        let den = self.c * z_out - self.a;
        if den == Complex64::new(0.0, 0.0) && num != Complex64::new(0.0, 0.0) {
            Complex64::new(f64::INFINITY, 0.0)
        } else {
            num / den
        }
    }
}

/// Ley de área de un segmento.
#[derive(Clone)]
pub enum Flare {
    /// Área constante.
    Cylindrical,
    /// Radio lineal: `S(x) = (r1 + (r2-r1)·x/L)²`.
    Conical,
    /// Área exponencial: `S(x) = S1·exp(m·x)`.
    Exponential,
    /// Área lineal (dos lados rectos paralelos): `S(x) = S1 + (S2-S1)·x/L`.
    ///
    /// No es una parábola matemática en el radio ni en el área: es el perfil
    /// de **área lineal** entre los extremos que el software de referencia
    /// etiqueta como `Par`. El radio resultante es `sqrt(S(x))`, que no es
    /// lineal.
    Parabolic,
    /// Familia de Salmon: `S(x) = S1·(cosh(x/xT) + T·sinh(x/xT))²`.
    /// `T = 0` es catenoidal; `T = 1/√2` reproduce el perfil de Le Cléac'h.
    HyperbolicExponential { t: f64 },
    /// Perfil arbitrario `S(x)` en m². Útil para tractrix, Bessel, OS, etc.
    ///
    /// La función debe devolver un área finita y estrictamente positiva para todo
    /// `x` en `[0, L]`. El motor no puede validar una closure arbitraria; una
    /// función no física produce resultados inválidos que `try_response_at`
    /// rechaza.
    Custom(Arc<dyn Fn(f64) -> f64 + Send + Sync>),
}

impl fmt::Debug for Flare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Flare::Cylindrical => write!(f, "Cylindrical"),
            Flare::Conical => write!(f, "Conical"),
            Flare::Exponential => write!(f, "Exponential"),
            Flare::Parabolic => write!(f, "Parabolic"),
            Flare::HyperbolicExponential { t } => {
                write!(f, "HyperbolicExponential{{t:{t}}}")
            }
            Flare::Custom(_) => write!(f, "Custom"),
        }
    }
}

impl Flare {
    /// Nombre corto (coincide con las etiquetas del software de referencia).
    pub fn short_name(&self) -> &'static str {
        match self {
            Flare::Cylindrical => "Cir",
            Flare::Conical => "Con",
            Flare::Exponential => "Exp",
            Flare::Parabolic => "Par",
            Flare::HyperbolicExponential { .. } => "Hyp",
            Flare::Custom(_) => "Cus",
        }
    }
}

/// Un tramo de guía de onda.
#[derive(Clone, Debug)]
pub struct Segment {
    flare: Flare,
    length: f64,
    s1: f64,
    s2: f64,
    /// Parámetro `xT` de la familia hiperbólico-exponencial (0 si no aplica).
    xt: f64,
    /// Pérdidas de pared (factor adimensional; 0 = sin pérdidas).
    pub loss: f64,
    flow_resistivity: f64,
}

impl Segment {
    /// Crea un segmento. `length` en m, `s1`/`s2` en m².
    ///
    /// El constructor no recorta valores: `length` debe ser finita y no
    /// negativa, y `s1`/`s2` finitas y positivas. Use [`Segment::validate`]
    /// antes de calcular con datos externos.
    pub fn new(flare: Flare, length: f64, s1: f64, s2: f64) -> Self {
        let mut seg = Self {
            flare,
            length,
            s1,
            s2,
            xt: 0.0,
            loss: 0.0,
            flow_resistivity: 0.0,
        };
        seg.xt = seg.solve_xt();
        seg
    }

    /// Resistencia distribuida serie Rf (Pa·s/m²), sin conversión desde densidad.
    /// Modelo lineal de Darcy homogéneo, no un ajuste universal de fibra.
    pub fn with_flow_resistivity(mut self, rf: f64) -> Self {
        self.flow_resistivity = rf;
        self
    }
    pub fn flow_resistivity(&self) -> f64 {
        self.flow_resistivity
    }
    pub fn has_distributed_losses(&self) -> bool {
        self.loss != 0.0 || self.flow_resistivity != 0.0
    }

    /// Comprueba la geometría del segmento.
    ///
    /// - `length` finita y no negativa (un tramo de longitud cero es la
    ///   identidad).
    /// - `s1` y `s2` finitas y estrictamente positivas.
    /// - En `HyperbolicExponential`, `t` finito, `|t| < 1`, expansión
    ///   para t>=0; para t<0 contracción hasta S1(1-t²), primera raíz.
    /// - Cilindro y tramo nulo requieren S1=S2; extremos relativos a 1e-10.
    /// - `loss` finito y no negativo.
    pub fn validate(&self) -> Result<(), crate::error::SolveError> {
        use crate::error::{require_finite, require_non_negative, require_positive};
        require_non_negative(self.length, "segment.length")?;
        require_positive(self.s1, "segment.s1")?;
        require_positive(self.s2, "segment.s2")?;
        if let Flare::HyperbolicExponential { t } = self.flare {
            require_finite(t, "segment.t")?;
            if t.abs() >= 1.0 {
                return Err(crate::error::SolveError::InvalidParameter("segment.t"));
            }
        }
        if (matches!(self.flare, Flare::Cylindrical) || self.length == 0.0) && self.s1 != self.s2 {
            return Err(crate::error::SolveError::InvalidParameter(
                "segment.endpoints",
            ));
        }
        if matches!(self.flare, Flare::HyperbolicExponential { .. })
            && (self.xt.is_nan() || self.xt <= 0.0)
        {
            return Err(crate::error::SolveError::InvalidParameter(
                "segment.hypex_domain",
            ));
        }
        // Los extremos deben coincidir; una closure se muestrea además en
        // cada paso RK4 (no hay demostración global para funciones arbitrarias).
        for (x, expected) in [(0.0, self.s1), (self.length, self.s2)] {
            let got = self.area(x);
            require_positive(got, "segment.area")?;
            if ((got - expected) / expected).abs() > 1.0e-10 {
                return Err(crate::error::SolveError::InvalidParameter(
                    "segment.endpoints",
                ));
            }
        }
        if matches!(self.flare, Flare::Custom(_)) {
            for i in 1..64 {
                require_positive(self.area(self.length * (i as f64 / 64.0)), "segment.area")?;
            }
        }
        require_non_negative(self.loss, "segment.loss")?;
        require_non_negative(self.flow_resistivity, "segment.flow_resistivity")?;
        Ok(())
    }

    /// Atajo para un segmento cónico.
    pub fn conical(length: f64, s1: f64, s2: f64) -> Self {
        Self::new(Flare::Conical, length, s1, s2)
    }

    /// Atajo para un segmento exponencial.
    pub fn exponential(length: f64, s1: f64, s2: f64) -> Self {
        Self::new(Flare::Exponential, length, s1, s2)
    }

    pub fn flare(&self) -> &Flare {
        &self.flare
    }

    pub fn length(&self) -> f64 {
        self.length
    }

    pub fn s1(&self) -> f64 {
        self.s1
    }

    pub fn s2(&self) -> f64 {
        self.s2
    }

    /// Área en la posición `x` (m), recortada al intervalo `[0, L]`.
    ///
    /// No impone un área mínima: un segmento inválido debe rechazarse con
    /// [`Segment::validate`] antes de calcular.
    pub fn area(&self, x: f64) -> f64 {
        let l = self.length;
        if l <= 0.0 {
            return self.s1;
        }
        let x = x.clamp(0.0, l);
        let xr = x / l;
        match &self.flare {
            Flare::Cylindrical => self.s1,
            Flare::Conical => {
                let r1 = self.s1.sqrt();
                let r2 = self.s2.sqrt();
                let r = r1 + (r2 - r1) * xr;
                r * r
            }
            Flare::Exponential => (self.s1.ln() * (1.0 - xr) + self.s2.ln() * xr).exp(),
            Flare::Parabolic => self.s1 + (self.s2 - self.s1) * xr,
            Flare::HyperbolicExponential { t } => {
                if self.xt == f64::INFINITY {
                    self.s1
                } else {
                    let shift = t.atanh();
                    let log_cosh =
                        |v: f64| v.abs() + (-2.0 * v.abs()).exp().ln_1p() - std::f64::consts::LN_2;
                    (self.s1.ln() + 2.0 * (log_cosh(x / self.xt + shift) - log_cosh(shift))).exp()
                }
            }
            // El perfil `Custom` es responsabilidad del consumidor: se
            // documenta que debe devolver un área finita y no negativa.
            Flare::Custom(f) => f(x),
        }
    }

    /// Frecuencia de corte del perfil (Hz). Es 0 para perfiles que no tienen
    /// frecuencia de corte (cilíndrico, cónico, parabólico).
    ///
    /// - Exponencial: `fc = c·ln(S2/S1)/(4π·L)`.
    /// - Hiperbólico-exponencial: `fc = c/(2π·xT)`.
    pub fn cutoff_hz(&self, air: &Air) -> f64 {
        match &self.flare {
            Flare::Exponential => {
                if self.length > 0.0 {
                    air.c * (self.s2 / self.s1).ln() / (4.0 * std::f64::consts::PI * self.length)
                } else {
                    0.0
                }
            }
            Flare::HyperbolicExponential { .. } if self.xt > 0.0 => {
                air.c / (2.0 * std::f64::consts::PI * self.xt)
            }
            Flare::HyperbolicExponential { .. } => 0.0,
            _ => 0.0,
        }
    }

    /// Volumen interior del segmento (m³), integrado con Simpson.
    pub fn volume(&self) -> f64 {
        let n = 512usize;
        let dx = self.length / n as f64;
        let mut sum = self.area(0.0) + self.area(self.length);
        for i in 1..n {
            let w = if i % 2 == 0 { 2.0 } else { 4.0 };
            sum += w * self.area(i as f64 * dx);
        }
        sum * dx / 3.0
    }

    fn solve_xt(&self) -> f64 {
        let Flare::HyperbolicExponential { t } = &self.flare else {
            return 0.0;
        };
        let t = *t;
        if !t.is_finite() || t.abs() >= 1.0 || self.length < 0.0 || self.s1 <= 0.0 || self.s2 <= 0.0
        {
            return f64::NAN;
        }
        if self.s1 == self.s2 {
            return f64::INFINITY;
        }
        if self.length == 0.0 {
            return f64::NAN;
        }
        // cosh(u)+t sinh(u)=sqrt(1-t²) cosh(u+atanh(t)).
        // t>=0 solo expande. t<0 admite contracción hasta S1(1-t²).
        // Para contracción se elige la primera raíz, sin cintura oculta.
        let log_ratio = 0.5 * (self.s2.ln() - self.s1.ln());
        let log_min = 0.5 * (-t * t).ln_1p();
        if log_ratio < 0.0 && (t >= 0.0 || log_ratio < log_min) {
            return f64::NAN;
        }
        let log_arg = log_ratio - log_min;
        let acosh = if log_arg > 350.0 {
            log_arg + std::f64::consts::LN_2
        } else {
            log_arg.exp().acosh()
        };
        let u = if log_ratio < 0.0 {
            -t.atanh() - acosh
        } else {
            -t.atanh() + acosh
        };
        if u > 0.0 {
            self.length / u
        } else {
            f64::NAN
        }
    }

    /// Derivadas de estado en `x` (Webster).
    #[inline]
    fn state_deriv(
        &self,
        x: f64,
        p: Complex64,
        u: Complex64,
        w: f64,
        rho: f64,
        c: f64,
    ) -> (Complex64, Complex64) {
        let area = self.area(x);
        let s = if area.is_finite() && area > 0.0 {
            area
        } else {
            f64::NAN
        };
        // Convenio físico: U positiva hacia el interior del segmento. Con
        // este signo, un tubo abierto presenta reactancia másica (+) a baja
        // frecuencia, como en la aplicación original.
        let dp = Complex64::new(-self.flow_resistivity, -w * rho) * u / s;
        // Pérdida de pared como conductancia en derivación: añade una parte
        // real que amortigua las resonancias de la bocina.
        let du = Complex64::new(-self.loss * w * s / (rho * c * c), -w * s / (rho * c * c)) * p;
        (dp, du)
    }

    /// Integra ∫g|p|² dx para la conductancia de pared g=loss·ωS/(ρc²).
    /// Usa las mismas etapas RK4 del estado, no una resta de flujos como pérdida.
    pub fn dissipated_power_w(
        &self,
        air: &Air,
        f: f64,
        mut p: Complex64,
        mut u: Complex64,
    ) -> Result<f64, crate::error::SolveError> {
        crate::error::check_frequency(f)?;
        self.validate()?;
        air.validate()?;
        if !self.has_distributed_losses() || self.length == 0.0 {
            return Ok(0.0);
        }
        let w = omega(f);
        let steps = self.integration_steps(air, f)? * 2;
        let dx = self.length / steps as f64;
        let q = |x: f64, p: Complex64, u: Complex64| {
            self.loss * w * self.area(x) / (air.rho * air.c * air.c) * p.norm_sqr()
                + self.flow_resistivity / self.area(x) * u.norm_sqr()
        };
        let mut loss = 0.0;
        for i in 0..steps {
            let x = i as f64 * dx;
            let (a, b) = self.state_deriv(x, p, u, w, air.rho, air.c);
            let p2 = p + 0.5 * dx * a;
            let u2 = u + 0.5 * dx * b;
            let (c, d) = self.state_deriv(x + 0.5 * dx, p2, u2, w, air.rho, air.c);
            let p3 = p + 0.5 * dx * c;
            let u3 = u + 0.5 * dx * d;
            let (e, g) = self.state_deriv(x + 0.5 * dx, p3, u3, w, air.rho, air.c);
            let p4 = p + dx * e;
            let u4 = u + dx * g;
            let (h, j) = self.state_deriv(x + dx, p4, u4, w, air.rho, air.c);
            loss += dx / 6.0
                * (q(x, p, u)
                    + 2.0 * q(x + dx / 2.0, p2, u2)
                    + 2.0 * q(x + dx / 2.0, p3, u3)
                    + q(x + dx, p4, u4));
            p += dx / 6.0 * (a + 2.0 * c + 2.0 * e + h);
            u += dx / 6.0 * (b + 2.0 * d + 2.0 * g + j);
        }
        if loss.is_finite() && loss >= 0.0 {
            Ok(loss)
        } else {
            Err(crate::error::SolveError::NonFiniteResponse)
        }
    }

    // Eigenvalues ±gamma of the local state operator. Area cancels in a*b.
    fn propagation_constant(&self, air: &Air, f: f64) -> Complex64 {
        let w = omega(f);
        (Complex64::new(self.flow_resistivity, w * air.rho) * Complex64::new(self.loss * w, w)
            / (air.rho * air.c * air.c))
            .sqrt()
    }

    fn integration_steps(&self, air: &Air, f: f64) -> Result<usize, crate::SolveError> {
        let g = self.propagation_constant(air, f) * self.length;
        // Transfer matrices carry growing AND decaying waves. exp(2 Re(g))
        // bounds their conditioning; beyond 8 nepers roundoff can erase the
        // transmitted wave. A scattering formulation is needed beyond this.
        if !g.norm().is_finite() || g.re.abs() > 8.0 || g.norm() / 0.03 > 30_000.0 {
            return Err(crate::SolveError::InvalidParameter(
                "segment propagation exceeds transfer-matrix conditioning domain",
            ));
        }
        Ok(((g.norm() / 0.03).ceil() as usize).max(192))
    }

    /// Checked propagation. Rejects >8 nepers attenuation per segment and
    /// unresolved profiles instead of returning a plausible inaccurate state.
    pub fn try_matrix(&self, air: &Air, f: f64) -> Result<TransferMatrix, crate::SolveError> {
        crate::error::check_frequency(f)?;
        air.validate()?;
        self.validate()?;
        let steps = self.integration_steps(air, f)?;
        if self.s1 == self.s2 && !matches!(self.flare, Flare::Custom(_)) {
            if !self.has_distributed_losses() {
                let kl = air.wave_number(f) * self.length;
                let z0 = air.rho * air.c / self.s1;
                return TransferMatrix {
                    a: kl.cos().into(),
                    b: Complex64::new(0., -z0 * kl.sin()),
                    c: Complex64::new(0., -kl.sin() / z0),
                    d: kl.cos().into(),
                }
                .checked();
            }
            let g = self.propagation_constant(air, f);
            let z = g * self.length;
            let sinhc = if z.norm() < 1e-5 {
                Complex64::new(1., 0.) + z * z / 6. + z.powu(4) / 120.
            } else {
                z.sinh() / z
            };
            let w = omega(f);
            return TransferMatrix {
                a: z.cosh(),
                d: z.cosh(),
                b: Complex64::new(-self.flow_resistivity, -w * air.rho) / self.s1
                    * self.length
                    * sinhc,
                c: Complex64::new(-self.loss * w, -w) * self.s1 / (air.rho * air.c * air.c)
                    * self.length
                    * sinhc,
            }
            .checked();
        }
        if !self.has_distributed_losses() {
            return Ok(self.rk4_matrix(air, f, steps));
        }
        let mut n = steps;
        let mut coarse = self.rk4_matrix(air, f, n);
        loop {
            n *= 2;
            let fine = self.rk4_matrix(air, f, n);
            let z0 = air.rho * air.c / (self.s1.sqrt() * self.s2.sqrt());
            let error = [
                (coarse.a, fine.a),
                (coarse.b / z0, fine.b / z0),
                (coarse.c * z0, fine.c * z0),
                (coarse.d, fine.d),
            ]
            .iter()
            .map(|(a, b)| (*a - *b).norm() / (1. + b.norm()))
            .fold(0., f64::max);
            if error.is_finite() && error < 1e-7 {
                return fine.checked();
            }
            if n >= 30_000 {
                return Err(crate::SolveError::InvalidResponse(
                    "segment RK4 did not converge",
                ));
            }
            coarse = fine;
        }
    }

    /// Matriz de transferencia del segmento a la frecuencia `freq_hz`.
    pub fn matrix(&self, air: &Air, freq_hz: f64) -> TransferMatrix {
        self.try_matrix(air, freq_hz).unwrap_or(TransferMatrix {
            a: Complex64::new(f64::NAN, f64::NAN),
            b: Complex64::new(f64::NAN, f64::NAN),
            c: Complex64::new(f64::NAN, f64::NAN),
            d: Complex64::new(f64::NAN, f64::NAN),
        })
    }

    fn rk4_matrix(&self, air: &Air, freq_hz: f64, steps: usize) -> TransferMatrix {
        let w = omega(freq_hz);
        let dx = self.length / steps as f64;

        // Dos estados base: [1,0] y [0,1].
        let mut pa = Complex64::new(1.0, 0.0);
        let mut ua = Complex64::new(0.0, 0.0);
        let mut pb = Complex64::new(0.0, 0.0);
        let mut ub = Complex64::new(1.0, 0.0);

        let mut x = 0.0;
        for _ in 0..steps {
            let (dpa1, dua1) = self.state_deriv(x, pa, ua, w, air.rho, air.c);
            let (dpb1, dub1) = self.state_deriv(x, pb, ub, w, air.rho, air.c);

            let (dpa2, dua2) = self.state_deriv(
                x + 0.5 * dx,
                pa + 0.5 * dx * dpa1,
                ua + 0.5 * dx * dua1,
                w,
                air.rho,
                air.c,
            );
            let (dpb2, dub2) = self.state_deriv(
                x + 0.5 * dx,
                pb + 0.5 * dx * dpb1,
                ub + 0.5 * dx * dub1,
                w,
                air.rho,
                air.c,
            );

            let (dpa3, dua3) = self.state_deriv(
                x + 0.5 * dx,
                pa + 0.5 * dx * dpa2,
                ua + 0.5 * dx * dua2,
                w,
                air.rho,
                air.c,
            );
            let (dpb3, dub3) = self.state_deriv(
                x + 0.5 * dx,
                pb + 0.5 * dx * dpb2,
                ub + 0.5 * dx * dub2,
                w,
                air.rho,
                air.c,
            );

            let (dpa4, dua4) =
                self.state_deriv(x + dx, pa + dx * dpa3, ua + dx * dua3, w, air.rho, air.c);
            let (dpb4, dub4) =
                self.state_deriv(x + dx, pb + dx * dpb3, ub + dx * dub3, w, air.rho, air.c);

            pa += (dx / 6.0) * (dpa1 + 2.0 * dpa2 + 2.0 * dpa3 + dpa4);
            ua += (dx / 6.0) * (dua1 + 2.0 * dua2 + 2.0 * dua3 + dua4);
            pb += (dx / 6.0) * (dpb1 + 2.0 * dpb2 + 2.0 * dpb3 + dpb4);
            ub += (dx / 6.0) * (dub1 + 2.0 * dub2 + 2.0 * dub3 + dub4);
            x += dx;
        }

        TransferMatrix {
            a: pa,
            b: pb,
            c: ua,
            d: ub,
        }
    }
}

/// Bound total attenuation, so subdivision cannot bypass conditioning checks.
pub(crate) fn check_cascade(
    segments: &[Segment],
    air: &Air,
    f: f64,
) -> Result<(), crate::SolveError> {
    let attenuation: f64 = segments
        .iter()
        .filter(|s| s.has_distributed_losses())
        .map(|s| (s.propagation_constant(air, f) * s.length).re.abs())
        .sum();
    if attenuation.is_finite() && attenuation <= 8.0 {
        Ok(())
    } else {
        Err(crate::SolveError::InvalidParameter(
            "cascade attenuation exceeds transfer-matrix conditioning domain",
        ))
    }
}

/// Conjunto de segmentos en serie que forman una bocina.
#[derive(Clone, Debug)]
pub struct Horn {
    pub segments: Vec<Segment>,
    /// Factor de escala aplicado a la impedancia de radiación de la boca.
    /// 1.0 = pistón en pantalla infinita; otros valores son calibración de carga,
    /// no una solución espacial exacta para otro ángulo sólido.
    pub radiation_scale: f64,
    /// Si es `true`, la boca se termina con una carga adaptada
    /// `rho*c/S_boca` geométrica (sin reflexión plana) en lugar del pistón.
    /// Requiere radiation_scale == mouth_radiation_area_factor.
    /// Es un absorbente ideal, no determina SPL; use System::try_energy_at.
    pub matched_mouth: bool,
    /// Factor de área efectiva para la impedancia de radiación de la boca.
    pub mouth_radiation_area_factor: f64,
}

impl Default for Horn {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            radiation_scale: 1.0,
            matched_mouth: false,
            mouth_radiation_area_factor: 1.0,
        }
    }
}

impl Horn {
    pub fn new(segments: Vec<Segment>) -> Self {
        Self {
            segments,
            radiation_scale: 1.0,
            matched_mouth: false,
            mouth_radiation_area_factor: 1.0,
        }
    }

    /// Comprueba que la bocina tenga segmentos válidos y factores físicos.
    pub fn validate(&self) -> Result<(), crate::error::SolveError> {
        use crate::error::{require_positive, SolveError};
        if self.segments.is_empty() {
            return Err(SolveError::InvalidTopology("bocina sin segmentos"));
        }
        for segment in &self.segments {
            segment.validate()?;
        }
        require_positive(self.radiation_scale, "horn.radiation_scale")?;
        require_positive(
            self.mouth_radiation_area_factor,
            "horn.mouth_radiation_area_factor",
        )?;
        if self.matched_mouth && self.radiation_scale != self.mouth_radiation_area_factor {
            return Err(SolveError::InvalidParameter(
                "matched_mouth requires radiation_scale == mouth_radiation_area_factor",
            ));
        }
        Ok(())
    }

    /// Área de la garganta (entrada) en m².
    pub fn throat_area(&self) -> f64 {
        self.segments.first().map(|s| s.s1()).unwrap_or(1.0e-4)
    }

    /// Área de la boca (salida) en m².
    pub fn mouth_area(&self) -> f64 {
        self.segments.last().map(|s| s.s2()).unwrap_or(1.0e-4)
    }

    /// Aplica un factor de pérdidas de pared a todos los segmentos.
    ///
    /// No recorta el valor: un factor negativo o no finito se rechaza en
    /// [`Horn::validate`].
    pub fn set_loss(&mut self, loss: f64) {
        for s in &mut self.segments {
            s.loss = loss;
        }
    }

    /// Longitud total en m.
    pub fn length(&self) -> f64 {
        self.segments.iter().map(|s| s.length()).sum()
    }

    /// Volumen interior total de la bocina (m³).
    pub fn volume(&self) -> f64 {
        self.segments.iter().map(|s| s.volume()).sum()
    }

    /// Matrices de todos los segmentos, de garganta a boca.
    pub fn matrices(&self, air: &Air, freq_hz: f64) -> Vec<TransferMatrix> {
        self.segments
            .iter()
            .map(|s| s.matrix(air, freq_hz))
            .collect()
    }

    /// Impedancia acústica de entrada de la bocina, cargada por la impedancia
    /// de radiación de la boca.
    pub fn input_impedance(&self, air: &Air, freq_hz: f64, solid_angle: f64) -> Complex64 {
        self.prepare(air, freq_hz, solid_angle).input_impedance
    }

    /// Calcula impedancia y transmite usando una única colección de matrices.
    pub fn transmit(
        &self,
        air: &Air,
        freq_hz: f64,
        solid_angle: f64,
        p_throat: Complex64,
        u_throat: Complex64,
    ) -> (Complex64, Complex64) {
        let prepared = self.prepare(air, freq_hz, solid_angle);
        (
            prepared.input_impedance,
            prepared.transmit(p_throat, u_throat),
        )
    }

    /// Carga de salida realmente utilizada por el solucionador (Pa·s/m³).
    /// En modo adaptado los factores deben cancelarse para conservar ρc/Sgeom.
    pub fn termination_impedance(&self, air: &Air, f: f64, solid_angle: f64) -> Complex64 {
        if self.matched_mouth {
            // Los factores iguales se cancelan algebraicamente antes de
            // multiplicar áreas extremas; no puede aparecer una carga cero.
            return Complex64::new(air.rho * air.c / self.mouth_area(), 0.0);
        }
        let area = self.mouth_area() * self.mouth_radiation_area_factor;
        piston_radiation_impedance(air, area, f, solid_angle) * self.radiation_scale
    }

    pub(crate) fn prepare(&self, air: &Air, f: f64, solid_angle: f64) -> PreparedHorn {
        let matrices = self.matrices(air, f);
        let termination_impedance = self.termination_impedance(air, f, solid_angle);
        let mut z = termination_impedance;
        for matrix in matrices.iter().rev() {
            z = matrix.transform_impedance(z);
        }
        PreparedHorn {
            matrices,
            input_impedance: z,
            termination_impedance,
        }
    }
}

/// Estado local de una sola frecuencia; no sobrevive a un cambio paramétrico.
pub(crate) struct PreparedHorn {
    pub(crate) matrices: Vec<TransferMatrix>,
    pub input_impedance: Complex64,
    pub termination_impedance: Complex64,
}
impl PreparedHorn {
    pub fn transmit(&self, mut p: Complex64, mut u: Complex64) -> Complex64 {
        for matrix in &self.matrices {
            (p, u) = matrix.apply(p, u);
        }
        u
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Air;

    #[test]
    fn cylindrical_matrix_matches_analytic() {
        let air = Air::default();
        let f = 1000.0;
        let s = 100.0e-4;
        let l = 0.3;
        let seg = Segment::new(Flare::Cylindrical, l, s, s);
        let m = seg.matrix(&air, f);
        let k = air.wave_number(f);
        let z0 = air.rho * air.c / s;
        let c = (k * l).cos();
        let sn = (k * l).sin();
        assert!((m.a - Complex64::new(c, 0.0)).norm() < 1.0e-7);
        assert!((m.d - Complex64::new(c, 0.0)).norm() < 1.0e-7);
        assert!((m.b - Complex64::new(0.0, -z0 * sn)).norm() / (z0 * sn).abs() < 1.0e-6);
        assert!((m.c - Complex64::new(0.0, -sn / z0)).norm() / (sn / z0).abs() < 1.0e-6);
    }

    #[test]
    fn matched_impedance_is_preserved() {
        let air = Air::default();
        let area = 100.0e-4;
        let z0 = air.rho * air.c / area;
        let seg = Segment::new(Flare::Cylindrical, 0.5, area, area);
        let z = seg
            .matrix(&air, 500.0)
            .transform_impedance(Complex64::new(z0, 0.0));
        assert!((z - Complex64::new(z0, 0.0)).norm() / z0 < 1.0e-9);
    }

    #[test]
    fn exponential_area_endpoints() {
        let seg = Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4);
        assert!((seg.area(0.0) - 350.0e-4).abs() < 1.0e-12);
        assert!((seg.area(0.75) - 2400.0e-4).abs() < 1.0e-12);
    }

    #[test]
    fn exponential_matrix_matches_analytic_webster() {
        // Matriz exacta de Webster para S(x)=S1·exp(m·x):
        //   beta = sqrt(k² - m²/4)
        //   A = e^(-mL/2)·(cos(beta L) + (m/(2 beta))·sin(beta L))
        //   B = j·e^(-mL/2)·(w rho/(S1 beta))·sin(beta L)
        //   C = j·e^(-mL/2)·(w S2/(rho c² beta))·sin(beta L)
        //   D = e^(+mL/2)·(cos(beta L) - (m/(2 beta))·sin(beta L))
        let air = Air::default();
        let s1: f64 = 350.0e-4;
        let s2: f64 = 2400.0e-4;
        let l: f64 = 0.75;
        let m = (s2 / s1).ln() / l;
        for f in [50.0_f64, 100.0, 500.0, 2000.0] {
            let seg = Segment::new(Flare::Exponential, l, s1, s2);
            let got = seg.matrix(&air, f);

            let k = air.wave_number(f);
            let beta = (Complex64::new(k * k - 0.25 * m * m, 0.0)).sqrt();
            let bl = beta * l;
            let cos = bl.cos();
            let sin = bl.sin();
            let e_neg = (-0.5 * m * l).exp();
            let e_pos = (0.5 * m * l).exp();
            let a = e_neg * (cos + (m / (2.0 * beta)) * sin);
            let b = Complex64::new(0.0, -omega(f) * air.rho) * e_neg / (s1 * beta) * sin;
            let c = Complex64::new(0.0, -omega(f) * s2) * e_neg / (air.rho * air.c * air.c * beta)
                * sin;
            let d = e_pos * (cos - (m / (2.0 * beta)) * sin);

            let rel = |x: Complex64, y: Complex64| (x - y).norm() / y.norm().max(1.0e-12);
            assert!(rel(got.a, a) < 1.0e-5, "A a {f} Hz");
            assert!(rel(got.b, b) < 1.0e-5, "B a {f} Hz");
            assert!(rel(got.c, c) < 1.0e-5, "C a {f} Hz");
            assert!(rel(got.d, d) < 1.0e-5, "D a {f} Hz");
        }
    }

    #[test]
    fn conical_matrix_matches_exact_solution() {
        let air = Air::default();
        let (l, s1, s2) = (0.35, 150.0e-4, 400.0e-4);
        let seg = Segment::new(Flare::Conical, l, s1, s2);
        let r1 = (s1 / std::f64::consts::PI).sqrt();
        let r2 = (s2 / std::f64::consts::PI).sqrt();
        let x0 = r1 * l / (r2 - r1);
        let z1 = x0;
        let z2 = x0 + l;

        for f in [100.0_f64, 500.0, 1000.0, 2000.0] {
            let k = air.wave_number(f);
            let w = omega(f);
            // C = j·S1/(ωρx0²), con U = C·(z·f' − f), p = f/z.
            let c = Complex64::new(0.0, 1.0) * s1 / (w * air.rho * x0 * x0);
            let mut exact = [Complex64::new(0.0, 0.0); 4];
            for (idx, (p1, u1)) in [
                (Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)),
                (Complex64::new(0.0, 0.0), Complex64::new(1.0, 0.0)),
            ]
            .iter()
            .enumerate()
            {
                let f1 = p1 * z1;
                let fp1 = u1 / (c * z1) + p1;
                let f2 = f1 * (k * l).cos() + (fp1 / k) * (k * l).sin();
                let fp2 = -f1 * k * (k * l).sin() + fp1 * (k * l).cos();
                let p2 = f2 / z2;
                let u2 = c * (z2 * fp2 - f2);
                if idx == 0 {
                    exact[0] = p2;
                    exact[2] = u2;
                } else {
                    exact[1] = p2;
                    exact[3] = u2;
                }
            }

            let got = seg.matrix(&air, f);
            let rel = |x: Complex64, y: Complex64| (x - y).norm() / y.norm().max(1.0e-12);
            assert!(rel(got.a, exact[0]) < 1.0e-6, "A a {f} Hz");
            assert!(rel(got.b, exact[1]) < 1.0e-6, "B a {f} Hz");
            assert!(rel(got.c, exact[2]) < 1.0e-6, "C a {f} Hz");
            assert!(rel(got.d, exact[3]) < 1.0e-6, "D a {f} Hz");
        }
    }

    #[test]
    fn exponential_cutoff_matches_reference() {
        let air = Air::default();
        let seg = Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4);
        // El registro por defecto del software de referencia muestra F23 = 70.27 Hz.
        assert!((seg.cutoff_hz(&air) - 70.27).abs() < 0.05);
    }

    #[test]
    fn horn_volume_is_positive() {
        let horn = Horn::new(vec![
            Segment::new(Flare::Conical, 0.60, 80.0e-4, 350.0e-4),
            Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4),
        ]);
        // Volumen geométrico ~91.8 L (el esquema de referencia muestra
        // 104.292 L incluyendo el recinto trasero de 14 L).
        assert!((horn.volume() * 1000.0 - 91.8).abs() < 2.0);
    }

    #[test]
    fn hypex_area_endpoints() {
        let seg = Segment::new(Flare::HyperbolicExponential { t: 0.707 }, 1.0, 0.01, 0.5);
        assert!((seg.area(0.0) - 0.01).abs() < 1.0e-9);
        assert!((seg.area(1.0) - 0.5).abs() < 1.0e-9);
    }

    #[test]
    fn segment_validation_rejects_invalid_geometry() {
        assert!(Segment::new(Flare::Conical, 0.5, 100.0e-4, 200.0e-4)
            .validate()
            .is_ok());
        assert!(Segment::new(Flare::Conical, -0.5, 100.0e-4, 200.0e-4)
            .validate()
            .is_err());
        assert!(Segment::new(Flare::Conical, 0.5, 0.0, 200.0e-4)
            .validate()
            .is_err());
        assert!(Segment::new(Flare::Conical, 0.5, f64::NAN, 200.0e-4)
            .validate()
            .is_err());

        let mut seg = Segment::new(Flare::Conical, 0.5, 100.0e-4, 200.0e-4);
        seg.loss = -1.0;
        assert!(seg.validate().is_err());

        let seg = Segment::new(
            Flare::HyperbolicExponential { t: 1.0 },
            0.5,
            100.0e-4,
            200.0e-4,
        );
        assert!(seg.validate().is_err());
    }

    #[test]
    fn horn_validation_rejects_empty_and_invalid_factors() {
        let mut horn = Horn::new(vec![Segment::new(Flare::Conical, 0.5, 100.0e-4, 200.0e-4)]);
        assert!(horn.validate().is_ok());
        horn.radiation_scale = 0.0;
        assert!(horn.validate().is_err());
        horn.radiation_scale = 1.0;
        horn.mouth_radiation_area_factor = f64::NAN;
        assert!(horn.validate().is_err());
        assert!(Horn::new(vec![]).validate().is_err());
    }
}

#[cfg(test)]
mod complex_propagation_tests {
    use super::*;
    #[test]
    fn analytic_complex_propagation_and_rk4_converge() {
        let air = Air::default();
        for rf in [0., 1., 1000., 100000.] {
            for (f, l) in [(10., 0.008), (100., 0.8), (1000., 0.08)] {
                let l = if rf > 10000. { l / 10. } else { l };
                let s = Segment::new(Flare::Cylindrical, l, 0.02, 0.02).with_flow_resistivity(rf);
                let exact = s.try_matrix(&air, f).unwrap();
                let a = s.rk4_matrix(&air, f, 32);
                let b = s.rk4_matrix(&air, f, 128);
                let e1 = (a.b - exact.b).norm();
                let e2 = (b.b - exact.b).norm();
                assert!(e2 < e1 / 100. || e2 < exact.b.norm() * 1e-12);
                assert!(e2 < exact.b.norm() * 1e-6);
                // Terminate resistively, then verify independent loss integral.
                let z = Complex64::new(air.rho * air.c / 0.02, 0.);
                let zin = exact.transform_impedance(z);
                let u = Complex64::new(1e-4, 0.);
                let p = zin * u;
                let (po, uo) = exact.apply(p, u);
                let loss = s.dissipated_power_w(&air, f, p, u).unwrap();
                let delta = (p * u.conj() - po * uo.conj()).re;
                assert!(delta >= -1e-10);
                assert!((delta - loss).abs() < 1e-8 + 1e-5 * loss);
            }
        }
    }
    #[test]
    fn variable_profile_refines_and_extreme_is_explicit() {
        let air = Air::default();
        let s = Segment::new(Flare::Conical, 0.8, 0.02, 0.04).with_flow_resistivity(1000.);
        let a = s.try_matrix(&air, 100.).unwrap();
        let b = s.rk4_matrix(&air, 100., 4096);
        assert!((a.b - b.b).norm() / b.b.norm() < 1e-7);
        let extreme = Segment::new(Flare::Cylindrical, 0.8, 0.02, 0.02).with_flow_resistivity(1e8);
        assert!(extreme.try_matrix(&air, 10.).is_err());
    }
}
