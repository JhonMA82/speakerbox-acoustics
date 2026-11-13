//! Bancos de drivers idénticos.
//!
//! Un [`Driver`] representa un altavoz simple. Para cajas dobles, torres con
//! varios drivers o configuraciones isobáricas se construye un banco y se
//! obtiene un **driver equivalente** que los modelos acústicos existentes
//! (`System`, `Bandpass`, `TappedHorn`, `OffsetHorn`) ya saben usar.
//!
//! ```rust
//! use speakerbox_acoustics::{Driver, DriverBank, Mechanical, Wiring};
//!
//! let base = Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1);
//! let bank = DriverBank::new(base, 4)
//!     .with_wiring(Wiring::SeriesParallel)
//!     .with_mechanical(Mechanical::Standard);
//! let driver = bank.equivalent_driver().unwrap();
//! assert_eq!(driver.sd, base.sd * 4.0);
//! ```
//!
//! El equivalente mantiene la misma área total, masa total, compliancia
//! equivalente y resistencia eléctrica del banco. Para la misma alineación,
//! el volumen de la caja escala con [`DriverBank::suggested_volume_scale`].
//!
//! Limitaciones actuales:
//! - `Bessel` requiere un modelo de array con posiciones y no se aproxima con
//!   un único driver equivalente.
//! - `DvcWiring::Separate` requiere dos canales independientes; use dos
//!   diseños separados o pase un `Driver` ya combinado con `dvc = None`.
//! - El array espacial (line array) necesita la capa `RadiatorArray`; este
//!   módulo solo calcula la respuesta axial de baja frecuencia del banco.

use crate::driver::Driver;

/// Cableado eléctrico del banco.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wiring {
    /// Todos los drivers en paralelo.
    Parallel,
    /// Todos los drivers en serie.
    Series,
    /// Pares en serie y esos pares en paralelo (convención de BassBox).
    SeriesParallel,
}

/// Configuración mecánica de los drivers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mechanical {
    /// Todos los drivers montados en la misma dirección.
    Standard,
    /// Pares isobáricos (compound): dos drivers por cámara isobárica.
    Isobaric,
    /// Igual que `Standard` en lineal; el push-pull cancela distorsión par.
    PushPull,
    /// Array Bessel de 5 o 6 drivers; requiere el modelo de array.
    Bessel,
}

/// Cableado de un driver de doble bobina (DVC).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DvcWiring {
    /// Bobinas en paralelo.
    Parallel,
    /// Bobinas en serie.
    Series,
    /// Una bobina por canal; no representable por un único driver.
    Separate,
}

/// Error de configuración de un banco.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BankError {
    /// Driver base o equivalente no físico.
    InvalidDriver,
    /// El número no cabe en el campo público nd.
    CountOutOfRange,
    /// El banco no puede tener cero drivers.
    ZeroDrivers,
    /// `SeriesParallel` necesita un número par de unidades.
    OddCountForSeriesParallel,
    /// `Isobaric` necesita un número par de drivers.
    IsobaricRequiresEven,
    /// `PushPull` necesita un número par de drivers.
    PushPullRequiresEven,
    /// `Isobaric` solo admite bobinas en serie o paralelo.
    InvalidIsobaricCoils,
    /// Bessel necesita el modelo de array.
    UnsupportedBessel,
    /// DVC Separate necesita dos canales independientes.
    UnsupportedDvcSeparate,
}

impl std::fmt::Display for BankError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::InvalidDriver => "driver base o equivalente inválido",
            Self::CountOutOfRange => "count excede u32::MAX",
            Self::ZeroDrivers => "un banco necesita al menos un driver",
            Self::OddCountForSeriesParallel => "series-parallel necesita un número par de unidades",
            Self::IsobaricRequiresEven => "isobaric necesita un número par de drivers",
            Self::PushPullRequiresEven => "push-pull necesita un número par de drivers",
            Self::InvalidIsobaricCoils => "isobaric solo admite bobinas en serie o en paralelo",
            Self::UnsupportedBessel => {
                "Bessel requiere el modelo de array, no un driver equivalente"
            }
            Self::UnsupportedDvcSeparate => "DVC Separate requiere dos canales independientes",
        };
        f.write_str(text)
    }
}

impl std::error::Error for BankError {}

/// Banco de drivers idénticos.
#[derive(Clone, Debug, PartialEq)]
pub struct DriverBank {
    /// Driver base; para DVC representa una bobina individual.
    pub driver: Driver,
    /// Número de drivers.
    pub count: usize,
    /// Cableado eléctrico del banco.
    pub wiring: Wiring,
    /// Configuración mecánica.
    pub mechanical: Mechanical,
    /// Cableado DVC del driver base, si es de doble bobina.
    pub dvc: Option<DvcWiring>,
    /// Cableado de las bobinas de cada par isobárico.
    pub isobaric_coils: Wiring,
}

impl DriverBank {
    /// Crea un banco con `count` drivers idénticos.
    pub fn new(driver: Driver, count: usize) -> Self {
        Self {
            driver,
            count,
            wiring: Wiring::Parallel,
            mechanical: Mechanical::Standard,
            dvc: None,
            isobaric_coils: Wiring::Parallel,
        }
    }

    /// Cambia el cableado eléctrico del banco.
    pub fn with_wiring(mut self, wiring: Wiring) -> Self {
        self.wiring = wiring;
        self
    }

    /// Cambia la configuración mecánica.
    pub fn with_mechanical(mut self, mechanical: Mechanical) -> Self {
        self.mechanical = mechanical;
        self
    }

    /// Indica el cableado DVC del driver base.
    pub fn with_dvc(mut self, dvc: DvcWiring) -> Self {
        self.dvc = Some(dvc);
        self
    }

    /// Cambia el cableado de las bobinas de cada par isobárico.
    pub fn with_isobaric_coils(mut self, coils: Wiring) -> Self {
        self.isobaric_coils = coils;
        self
    }

    /// Driver equivalente del banco.
    ///
    /// Los parámetros mecánicos y eléctricos ya incluyen el número de drivers,
    /// el cableado y la configuración mecánica. El `nd` resultante es el número
    /// de drivers físicos.
    pub fn equivalent_driver(&self) -> Result<Driver, BankError> {
        if self.count == 0 {
            return Err(BankError::ZeroDrivers);
        }
        if self.mechanical == Mechanical::Bessel {
            return Err(BankError::UnsupportedBessel);
        }
        if self.mechanical == Mechanical::PushPull && self.count % 2 != 0 {
            return Err(BankError::PushPullRequiresEven);
        }

        self.driver
            .validate()
            .map_err(|_| BankError::InvalidDriver)?;
        let count = u32::try_from(self.count).map_err(|_| BankError::CountOutOfRange)?;
        let base = self.apply_dvc()?;
        let (unit, units) = self.apply_mechanical(base)?;

        let mut eq = unit;
        let units_f = units as f64;
        match self.wiring {
            Wiring::Parallel => {
                eq.re /= units_f;
                eq.le /= units_f;
            }
            Wiring::Series => {
                eq.re *= units_f;
                eq.le *= units_f;
                eq.bl *= units_f;
            }
            Wiring::SeriesParallel => {
                if units < 4 || units % 2 != 0 {
                    return Err(BankError::OddCountForSeriesParallel);
                }
                let s = 2.0;
                let p = units_f / 2.0;
                eq.re *= s / p;
                eq.le *= s / p;
                eq.bl *= s;
            }
        }

        eq.sd *= units_f;
        eq.mmd *= units_f;
        eq.cms /= units_f;
        eq.rms *= units_f;
        eq.nd = count;
        eq.validate().map_err(|_| BankError::InvalidDriver)?;
        Ok(eq)
    }

    /// Resistencia DC en frío que ve el amplificador (Ω).
    pub fn bank_resistance(&self) -> Result<f64, BankError> {
        Ok(self.equivalent_driver()?.re)
    }

    /// Factor de volumen de caja para conservar la alineación.
    ///
    /// - Standard / push-pull: `N` (cada driver necesita su volumen).
    /// - Isobaric: `N/4` (cada par necesita la mitad; hay `N/2` pares).
    ///
    /// Es una guía, no una obligación: el diseñador puede elegir otro volumen.
    pub fn suggested_volume_scale(&self) -> Result<f64, BankError> {
        if self.count == 0 {
            return Err(BankError::ZeroDrivers);
        }
        match self.mechanical {
            Mechanical::Standard | Mechanical::PushPull => Ok(self.count as f64),
            Mechanical::Isobaric => Ok(self.count as f64 / 4.0),
            Mechanical::Bessel => Err(BankError::UnsupportedBessel),
        }
    }

    fn apply_dvc(&self) -> Result<Driver, BankError> {
        let mut d = self.driver;
        match self.dvc {
            None => {}
            Some(DvcWiring::Parallel) => {
                d.re *= 0.5;
                d.le *= 0.5;
            }
            Some(DvcWiring::Series) => {
                d.re *= 2.0;
                d.le *= 2.0;
                d.bl *= 2.0;
            }
            Some(DvcWiring::Separate) => return Err(BankError::UnsupportedDvcSeparate),
        }
        Ok(d)
    }

    /// Devuelve el driver de una unidad (par isobárico o driver simple) y
    /// cuántas unidades hay en el banco.
    fn apply_mechanical(&self, base: Driver) -> Result<(Driver, usize), BankError> {
        match self.mechanical {
            Mechanical::Standard | Mechanical::PushPull => Ok((base, self.count)),
            Mechanical::Isobaric => {
                if self.count % 2 != 0 {
                    return Err(BankError::IsobaricRequiresEven);
                }
                let coils = self.isobaric_coils;
                if coils == Wiring::SeriesParallel {
                    return Err(BankError::InvalidIsobaricCoils);
                }
                let pair = isobaric_pair(base, coils)?;
                Ok((pair, self.count / 2))
            }
            Mechanical::Bessel => Err(BankError::UnsupportedBessel),
        }
    }
}

/// Driver equivalente de un par isobárico (compound).
///
/// Dos drivers comparten una cámara isobárica y actúan como un pistón de área
/// `Sd`: la masa móvil se duplica, la compliancia se reduce a la mitad y la
/// resistencia mecánica se duplica. Las bobinas pueden ir en serie o en
/// paralelo.
pub fn isobaric_pair(base: Driver, coils: Wiring) -> Result<Driver, BankError> {
    if coils == Wiring::SeriesParallel {
        return Err(BankError::InvalidIsobaricCoils);
    }
    let mut d = base;
    d.mmd *= 2.0;
    d.cms *= 0.5;
    d.rms *= 2.0;
    match coils {
        Wiring::Parallel => {
            d.re *= 0.5;
            d.le *= 0.5;
        }
        Wiring::Series => {
            d.re *= 2.0;
            d.le *= 2.0;
            d.bl *= 2.0;
        }
        Wiring::SeriesParallel => unreachable!(),
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Driver {
        Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 2)
    }

    fn assert_close(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol * b.abs().max(1.0), "{a} vs {b}");
    }

    #[test]
    fn two_parallel_drivers_scale() {
        let b = base();
        let bank = DriverBank::new(b, 2).with_wiring(Wiring::Parallel);
        let d = bank.equivalent_driver().unwrap();
        assert_close(d.sd, b.sd * 2.0, 1.0e-12);
        assert_close(d.mmd, b.mmd * 2.0, 1.0e-12);
        assert_close(d.cms, b.cms / 2.0, 1.0e-12);
        assert_close(d.rms, b.rms * 2.0, 1.0e-12);
        assert_close(d.re, b.re / 2.0, 1.0e-12);
        assert_close(d.le, b.le / 2.0, 1.0e-12);
        assert_close(d.bl, b.bl, 1.0e-12);
        assert_eq!(d.nd, 2);
        assert_close(bank.bank_resistance().unwrap(), b.re / 2.0, 1.0e-12);
    }

    #[test]
    fn two_series_drivers_scale() {
        let b = base();
        let bank = DriverBank::new(b, 2).with_wiring(Wiring::Series);
        let d = bank.equivalent_driver().unwrap();
        assert_close(d.sd, b.sd * 2.0, 1.0e-12);
        assert_close(d.re, b.re * 2.0, 1.0e-12);
        assert_close(d.le, b.le * 2.0, 1.0e-12);
        assert_close(d.bl, b.bl * 2.0, 1.0e-12);
        assert_close(bank.bank_resistance().unwrap(), b.re * 2.0, 1.0e-12);
    }

    #[test]
    fn four_series_parallel_drivers_scale() {
        let b = base();
        let bank = DriverBank::new(b, 4).with_wiring(Wiring::SeriesParallel);
        let d = bank.equivalent_driver().unwrap();
        assert_close(d.sd, b.sd * 4.0, 1.0e-12);
        assert_close(d.re, b.re, 1.0e-12); // dos pares en serie, en paralelo
        assert_close(d.le, b.le, 1.0e-12);
        assert_close(d.bl, b.bl * 2.0, 1.0e-12);
        assert_close(bank.bank_resistance().unwrap(), b.re, 1.0e-12);
    }

    #[test]
    fn isobaric_pair_scales_mass_and_compliance() {
        let b = base();
        let bank = DriverBank::new(b, 2)
            .with_mechanical(Mechanical::Isobaric)
            .with_isobaric_coils(Wiring::Parallel);
        let d = bank.equivalent_driver().unwrap();
        assert_close(d.sd, b.sd, 1.0e-12);
        assert_close(d.mmd, b.mmd * 2.0, 1.0e-12);
        assert_close(d.cms, b.cms / 2.0, 1.0e-12);
        assert_close(d.rms, b.rms * 2.0, 1.0e-12);
        assert_close(d.re, b.re / 2.0, 1.0e-12);
        assert_close(d.bl, b.bl, 1.0e-12);
        assert_close(bank.suggested_volume_scale().unwrap(), 0.5, 1.0e-12);
    }

    #[test]
    fn four_isobaric_drivers_are_two_pairs() {
        let b = base();
        let bank = DriverBank::new(b, 4)
            .with_mechanical(Mechanical::Isobaric)
            .with_isobaric_coils(Wiring::Series)
            .with_wiring(Wiring::Parallel);
        let d = bank.equivalent_driver().unwrap();
        assert_close(d.sd, b.sd * 2.0, 1.0e-12); // dos pares
        assert_close(d.mmd, b.mmd * 4.0, 1.0e-12);
        assert_close(d.cms, b.cms / 4.0, 1.0e-12);
        assert_close(d.re, b.re, 1.0e-12); // cada par 2Re, dos en paralelo
        assert_close(d.bl, b.bl * 2.0, 1.0e-12);
        assert_close(bank.suggested_volume_scale().unwrap(), 1.0, 1.0e-12);
    }

    #[test]
    fn push_pull_is_linear_equivalent_to_standard() {
        let b = base();
        let standard = DriverBank::new(b, 4)
            .with_wiring(Wiring::SeriesParallel)
            .equivalent_driver()
            .unwrap();
        let push_pull = DriverBank::new(b, 4)
            .with_mechanical(Mechanical::PushPull)
            .with_wiring(Wiring::SeriesParallel)
            .equivalent_driver()
            .unwrap();
        assert_eq!(standard, push_pull);
    }

    #[test]
    fn dvc_parallel_and_series() {
        let b = base();
        let parallel = DriverBank::new(b, 1)
            .with_dvc(DvcWiring::Parallel)
            .equivalent_driver()
            .unwrap();
        assert_close(parallel.re, b.re / 2.0, 1.0e-12);
        assert_close(parallel.bl, b.bl, 1.0e-12);

        let series = DriverBank::new(b, 1)
            .with_dvc(DvcWiring::Series)
            .equivalent_driver()
            .unwrap();
        assert_close(series.re, b.re * 2.0, 1.0e-12);
        assert_close(series.bl, b.bl * 2.0, 1.0e-12);
    }

    #[test]
    fn invalid_counts_are_rejected() {
        let b = base();
        assert_eq!(
            DriverBank::new(b, 0).equivalent_driver(),
            Err(BankError::ZeroDrivers)
        );
        assert_eq!(
            DriverBank::new(b, 2)
                .with_wiring(Wiring::SeriesParallel)
                .equivalent_driver(),
            Err(BankError::OddCountForSeriesParallel)
        );
        assert_eq!(
            DriverBank::new(b, 3)
                .with_mechanical(Mechanical::Isobaric)
                .equivalent_driver(),
            Err(BankError::IsobaricRequiresEven)
        );
        assert_eq!(
            DriverBank::new(b, 3)
                .with_mechanical(Mechanical::PushPull)
                .equivalent_driver(),
            Err(BankError::PushPullRequiresEven)
        );
        assert_eq!(
            DriverBank::new(b, 2)
                .with_mechanical(Mechanical::Isobaric)
                .with_isobaric_coils(Wiring::SeriesParallel)
                .equivalent_driver(),
            Err(BankError::InvalidIsobaricCoils)
        );
        assert_eq!(
            DriverBank::new(b, 1)
                .with_dvc(DvcWiring::Separate)
                .equivalent_driver(),
            Err(BankError::UnsupportedDvcSeparate)
        );
    }
}
