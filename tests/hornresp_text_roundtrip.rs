use speakerbox_acoustics::{record::HornrespTextRecord, CoilModel};

#[test]
fn nominal_sealed_and_vented_templates_round_trip() {
    for template in [
        include_str!("data/sealed_a_record.txt"),
        include_str!("data/vented_a_record.txt"),
    ] {
        let original = HornrespTextRecord::parse(template).unwrap();
        assert_eq!(original.original_text().as_bytes(), template.as_bytes());
        let mut system = original.simple_system().unwrap();
        assert_eq!(original.export_simple_system(&system).unwrap(), template);
        system.eg = 3.127;
        system.rg = 0.75;
        system.driver.le = 0.000634;
        system.rear_chamber.as_mut().unwrap().volume = 0.03873;
        let output = original.export_simple_system(&system).unwrap();
        let reader = HornrespTextRecord::parse(&output).unwrap();
        assert_eq!(
            reader.value("MAXIMUM SPL PARAMETER VALUES", "Xmax"),
            original.value("MAXIMUM SPL PARAMETER VALUES", "Xmax")
        );
        assert_eq!(
            reader.value("CHAMBER PARAMETER VALUES", "Lrc"),
            original.value("CHAMBER PARAMETER VALUES", "Lrc")
        );
        assert_eq!(output.lines().count(), template.lines().count());
        let result = reader.simple_system().unwrap();
        assert_eq!(result.eg, system.eg);
        assert_eq!(result.rg, system.rg);
        assert!((result.driver.le - system.driver.le).abs() < 1e-16);
        assert!(
            (result.rear_chamber.unwrap().volume - system.rear_chamber.unwrap().volume).abs()
                < 1e-15
        );
        assert_eq!(result.port, system.port);
        system.spl_offset_db = 0.1;
        assert!(original.export_simple_system(&system).is_err());
    }
}

#[test]
fn advanced_record_reads_distinct_fields_and_preserves_lexical_values() {
    let text="ID = 60.70\r\n|TRADITIONAL DRIVER PARAMETER VALUES:\r\nLe = 1.20\r\n|ADVANCED DRIVER PARAMETER VALUES FOR SEMI-INDUCTANCE MODEL:\r\nRe' = 5,9\r\nLeb = 0.30\r\nLe = 1.0\r\nKe = 0.05\r\nRss = 30\r\n|ADVANCED DRIVER PARAMETER VALUES FOR FREQUENCY-DEPENDENT DAMPING MODEL:\r\nRms = 2.0\r\nAms = 3.0E-4\r\n|STATUS FLAGS:\r\nSemi-Inductance Model Flag = 1\r\nDamping Model Flag = 1\r\n";
    let record = HornrespTextRecord::parse(text).unwrap();
    assert_eq!(record.original_text(), text);
    assert_eq!(
        record
            .number("TRADITIONAL DRIVER PARAMETER VALUES", "Le")
            .unwrap(),
        1.2
    );
    assert_eq!(
        record.advanced_coil().unwrap(),
        Some(CoilModel::ExtendedSemiInductance {
            re_prime_ohm: 5.9,
            leb_h: 0.0003,
            le_h: 0.001,
            rss_ohm: 30.0,
            ke_ohm_sqrt_s: 0.05,
        })
    );
    let fdd = record.mechanical_damping().unwrap().unwrap();
    assert_eq!(fdd.rms_kg_s, 2.);
    assert_eq!(fdd.compliance_loss_m_n, 0.0003);
    assert!(record.simple_system().is_err());
}
