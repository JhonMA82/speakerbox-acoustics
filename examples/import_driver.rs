//! Importa un driver WinISD `.wdr` o OpenISD `.owdr`.
//!
//! ```text
//! cargo run --release --example import_driver -- ruta/driver.wdr
//! cargo run --release --example import_driver -- ruta/driver.owdr
//! ```

use speakerbox_acoustics::import_driver_file;

fn main() {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("uso: import_driver <archivo.wdr|archivo.owdr>");
        std::process::exit(2);
    };
    match import_driver_file(&path) {
        Ok(imported) => {
            let d = imported.driver;
            println!("formato: {:?}", imported.format);
            println!("marca:   {}", imported.brand);
            println!("modelo:  {}", imported.model);
            println!("bobinas: {}", imported.voice_coils);
            println!("Sd:      {:.6} m²", d.sd);
            println!("Cms:     {:.6e} m/N", d.cms);
            println!("Mmd:     {:.6} kg", d.mmd);
            println!("Re:      {:.3} Ω", d.re);
            println!("Bl:      {:.3} T·m", d.bl);
            println!("Rms:     {:.3} kg/s", d.rms);
            println!("Le:      {:.6} H", d.le);
            if let Some(xmax) = imported.xmax_mm {
                println!("Xmax:    {xmax:.2} mm");
            }
            if let Some(pe) = imported.pe_w {
                println!("Pe:      {pe:.1} W");
            }
            if !imported.warnings.is_empty() {
                println!("avisos:");
                for warning in imported.warnings {
                    println!("  - {warning}");
                }
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
