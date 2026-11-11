//! Shows the error overlay for a few seconds (to look at it): `alert <seconds>`.

fn main() -> Result<(), String> {
    let seconds: u64 = std::env::args().nth(1).and_then(|text| text.parse().ok()).unwrap_or(8);
    let tray = qnc_tray::Tray::start("QNC AI alert", vec![])?;
    tray.alert("QNC AI", "Vratite karticu XAVC (D:\\): koristi se.\n\nKartica je izvađena bez izbacivanja, a priča je još treba. Za izlazak kartice koristite \"Izbaci\" u izborniku QNC AI.", "U redu");
    std::thread::sleep(std::time::Duration::from_secs(seconds));
    Ok(())
}
