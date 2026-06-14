//! Strumenti di rete locali: controllo porte, ping, traceroute, Wake-on-LAN.
//! ping/traceroute usano i comandi di sistema (presenti su tutti gli OS).

use std::time::Duration;

use tokio::net::TcpStream;
use tokio::time::timeout;

/// Verifica se una porta TCP è raggiungibile (timeout 3s).
pub async fn porta_aperta(host: &str, porta: u16) -> bool {
    matches!(
        timeout(Duration::from_secs(3), TcpStream::connect((host, porta))).await,
        Ok(Ok(_))
    )
}

/// Esegue un comando di sistema e ne restituisce l'output (stdout+stderr).
fn comando(prog: &str, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new(prog)
        .args(args)
        .output()
        .map_err(|e| format!("'{prog}' non disponibile: {e}"))?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok(s)
}

/// Ping verso un host (4 pacchetti).
pub fn ping(host: &str) -> Result<String, String> {
    #[cfg(windows)]
    let args = vec!["-n", "4", host];
    #[cfg(not(windows))]
    let args = vec!["-c", "4", host];
    comando("ping", &args)
}

/// Traceroute verso un host.
pub fn traceroute(host: &str) -> Result<String, String> {
    #[cfg(windows)]
    {
        comando("tracert", &[host])
    }
    #[cfg(not(windows))]
    {
        comando("traceroute", &[host])
    }
}

/// Wake-on-LAN: invia il "magic packet" all'indirizzo MAC indicato.
pub fn wake_on_lan(mac: &str) -> Result<(), String> {
    let bytes = parse_mac(mac)?;
    let mut pkt = vec![0xFFu8; 6];
    for _ in 0..16 {
        pkt.extend_from_slice(&bytes);
    }
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").map_err(|e| e.to_string())?;
    sock.set_broadcast(true).map_err(|e| e.to_string())?;
    sock.send_to(&pkt, "255.255.255.255:9")
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_mac(mac: &str) -> Result<[u8; 6], String> {
    let parti: Vec<&str> = mac.split(|c| c == ':' || c == '-').collect();
    if parti.len() != 6 {
        return Err("MAC non valido (atteso formato AA:BB:CC:DD:EE:FF)".into());
    }
    let mut b = [0u8; 6];
    for (i, p) in parti.iter().enumerate() {
        b[i] = u8::from_str_radix(p, 16).map_err(|_| "MAC non valido".to_string())?;
    }
    Ok(b)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mac_valido() {
        assert!(parse_mac("AA:BB:CC:DD:EE:FF").is_ok());
        assert!(parse_mac("aa-bb-cc-dd-ee-ff").is_ok());
        assert!(parse_mac("nope").is_err());
        assert!(parse_mac("AA:BB:CC").is_err());
    }
}
