//! Recorrido mínimo por todas las topologías del motor.
//!
//! ```text
//! cargo run --release --example quickstart
//! ```
//!
//! Cada bloque construye un diseño distinto y consulta la respuesta a 50 Hz.
//! Los diseños son los mismos que usan los tests de producción, así que sirven
//! como punto de partida para una aplicación real.

use speakerbox_acoustics::{
    Air, Bandpass, Chamber, Design, Driver, DriverBank, EndCorrection, Flare, Horn, OffsetHorn,
    PassiveRadiator, Port, Segment, System, TappedHorn, Wiring,
};

fn driver() -> Driver {
    Driver::from_ui_units(350.0, 4.0e-4, 20.0, 6.0, 18.0, 4.0, 1.0, 1)
}

fn port(area_cm2: f64, length_cm: f64) -> Port {
    Port::new(
        area_cm2 * 1.0e-4,
        length_cm * 1.0e-2,
        EndCorrection::UnflangedBoth,
    )
}

fn main() {
    let air = Air::default();

    // --- Radiador directo: sellada, ventilada y radiador pasivo -----------
    let mut sealed = System::new(air, driver());
    sealed.solid_angle = std::f64::consts::TAU;
    sealed.rear_chamber = Some(Chamber::new(50.0e-3));

    let mut vented = System::new(air, driver());
    vented.solid_angle = std::f64::consts::TAU;
    vented.rear_chamber = Some(Chamber::new(100.0e-3));
    vented.port = Some(Port::new(100.0e-4, 0.10, EndCorrection::Inner(1.5)));
    vented.rear_ql = Some(7.0);

    let mut passive = System::new(air, driver());
    passive.solid_angle = std::f64::consts::TAU;
    passive.rear_chamber = Some(Chamber::new(100.0e-3));
    passive.passive_radiator = Some(PassiveRadiator::new(350.0e-4, 0.4, 40.0e-3, 4.0));

    // --- Cuatro drivers en serie-paralelo, mismo recinto -----------------
    let bank = DriverBank::new(driver(), 4).with_wiring(Wiring::SeriesParallel);
    let mut four_drivers = System::new(air, bank.equivalent_driver().expect("banco"));
    four_drivers.solid_angle = std::f64::consts::TAU;
    four_drivers.rear_chamber = Some(Chamber::new(40.0e-3));

    // --- Bocina normal ----------------------------------------------------
    let mut horn = System::new(air, driver());
    horn.solid_angle = 0.5 * std::f64::consts::PI;
    horn.horn = Some(Horn::new(vec![
        Segment::new(Flare::Conical, 0.60, 80.0e-4, 350.0e-4),
        Segment::new(Flare::Exponential, 0.75, 350.0e-4, 2400.0e-4),
    ]));
    horn.throat_chamber = Some(Chamber::new(4.0e-3));
    horn.rear_chamber = Some(Chamber::new(14.0e-3));

    // --- Bandpass ---------------------------------------------------------
    let bp4 = Bandpass::new_bp4(driver(), 20.0e-3, 10.0e-3, port(50.0, 10.0));
    let bp6p = Bandpass::new_bp6(
        driver(),
        20.0e-3,
        10.0e-3,
        port(50.0, 10.0),
        port(40.0, 8.0),
    );
    let bp6s = Bandpass::new_bp6_series(
        driver(),
        20.0e-3,
        10.0e-3,
        port(50.0, 10.0),
        port(40.0, 8.0),
    );

    // --- Tapped horn TH y TH1 --------------------------------------------
    let th_segments = vec![
        Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
        Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
        Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
        Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
    ];
    let th = TappedHorn::new_4seg_th(driver(), th_segments.clone());
    let th1 = TappedHorn::new_4seg_th1(driver(), th_segments);

    // --- Offset driver OD / OD1 / OD2 ------------------------------------
    let od = OffsetHorn::new_4seg_od(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.40, 400e-4, 400e-4),
            Segment::new(Flare::Exponential, 0.20, 400e-4, 400e-4),
            Segment::new(Flare::Conical, 1.20, 400e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1500e-4),
        ],
    );
    let od1 = OffsetHorn::new_4seg_od1(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.25, 300e-4, 500e-4),
            Segment::new(Flare::Conical, 0.35, 500e-4, 500e-4),
            Segment::new(Flare::Conical, 0.90, 500e-4, 1200e-4),
            Segment::new(Flare::Conical, 0.30, 1200e-4, 1600e-4),
        ],
    );
    let od2 = OffsetHorn::new_4seg_od2(
        driver(),
        vec![
            Segment::new(Flare::Conical, 0.20, 250e-4, 450e-4),
            Segment::new(Flare::Exponential, 0.30, 450e-4, 700e-4),
            Segment::new(Flare::Conical, 0.90, 700e-4, 700e-4),
            Segment::new(Flare::Conical, 0.60, 700e-4, 1600e-4),
        ],
    );

    let designs = [
        ("sellada", Design::System(sealed)),
        ("ventilada", Design::System(vented)),
        ("rad. pasivo", Design::System(passive)),
        ("4 drivers", Design::System(four_drivers)),
        ("bocina", Design::System(horn)),
        ("BP4", Design::Bandpass(bp4)),
        ("BP6P", Design::Bandpass(bp6p)),
        ("BP6S", Design::Bandpass(bp6s)),
        ("TH", Design::TappedHorn(th)),
        ("TH1", Design::TappedHorn(th1)),
        ("OD", Design::OffsetHorn(od)),
        ("OD1", Design::OffsetHorn(od1)),
        ("OD2", Design::OffsetHorn(od2)),
    ];

    println!(
        "{:<12} {:>10} {:>10} {:>10} {:>12}",
        "diseño", "SPL(dB)", "Ze(Ω)", "Xd(mm)", "v_puerto(m/s)"
    );
    for (name, design) in designs {
        // `response_at` devuelve un punto neutro si el diseño no es resoluble.
        // Para distinguir errores, use `try_response_at`.
        let p = design.response_at(50.0);
        println!(
            "{:<12} {:>10.2} {:>10.2} {:>10.3} {:>12.2}",
            name,
            p.spl_db,
            p.ze_mag,
            p.xd_mm,
            p.port_velocity.unwrap_or(0.0)
        );
    }
}
