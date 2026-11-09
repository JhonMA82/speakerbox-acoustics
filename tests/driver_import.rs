//! API pública de importación de drivers WinISD/OpenISD.

use speakerbox_acoustics::{import_driver, import_owdr, import_wdr, DriverFormat};

const WDR: &str = "[Driver]\r\n\
Brand=Test\r\nModel=W1\r\n\
Fs=50\r\nQms=2\r\nQes=0.5\r\nVas=0.05\r\nRe=6\r\nSd=0.02\r\n\
Mms=0.03\r\nCms=0.0004\r\nBL=10\r\nRms=2\r\nLe=0.001\r\n";

const OWDR: &str = r#"{
  "brand": {"value": "Test"},
  "model": {"value": "O1"},
  "specs": {
    "woofer": {
      "Fs_hz": {"value": 50.0},
      "Qms": {"value": 2.0},
      "Qes": {"value": 0.5},
      "Vas_m3": {"value": 0.05},
      "Re_ohm": {"value": 6.0},
      "Sd_m2": {"value": 0.02},
      "Mms_kg": {"value": 0.03},
      "Cms_m_per_N": {"value": 0.0004},
      "BL_Tm": {"value": 10.0},
      "Rms_kg_per_s": {"value": 2.0},
      "Le_H": {"value": 0.001}
    }
  }
}"#;

#[test]
fn public_wdr_api() {
    let imported = import_wdr(WDR).expect("wdr");
    assert_eq!(imported.format, DriverFormat::WinIsd);
    assert_eq!(imported.brand, "Test");
    assert_eq!(imported.model, "W1");
    assert!((imported.driver.sd - 0.02).abs() < 1.0e-15);
}

#[test]
fn public_owdr_api() {
    let imported = import_owdr(OWDR).expect("owdr");
    assert_eq!(imported.format, DriverFormat::OpenIsd);
    assert!((imported.driver.cms - 4.0e-4).abs() < 1.0e-15);
}

#[test]
fn public_auto_detect() {
    assert_eq!(import_driver(WDR).unwrap().format, DriverFormat::WinIsd);
    assert_eq!(import_driver(OWDR).unwrap().format, DriverFormat::OpenIsd);
}
