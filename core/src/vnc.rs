//! Client VNC (RFB) — SPERIMENTALE.
//!
//! Si collega a un server VNC, chiede aggiornamenti dello schermo e li manda al
//! frontend come rettangoli RGBA (che la UI disegna su un canvas). Inoltra mouse
//! e tastiera. Per semplicità usa solo l'encoding Raw (nessun framebuffer locale).
//!
//! Limiti noti: niente Zrle/CopyRect (più banda), input elaborato a ~frame, e
//! non è stato possibile provarlo dal vivo: va considerato un primo abbozzo.

use std::net::TcpStream;

use tokio::sync::mpsc;
use vnc::client::{AuthChoice, AuthMethod, Client, Event};
use vnc::{Encoding, PixelFormat, Rect};

/// Comando inviato al client VNC dalla UI.
pub enum ComandoVnc {
    /// Posizione del mouse e maschera dei bottoni premuti.
    Mouse { x: u16, y: u16, bottoni: u8 },
    /// Tasto (keysym X11) premuto o rilasciato.
    Tasto { giu: bool, key: u32 },
    Chiudi,
}

/// Un rettangolo di schermo aggiornato (RGBA), o un evento di ridimensionamento.
pub struct FrameVnc {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
    pub rgba: Vec<u8>,
    pub resize: Option<(u16, u16)>,
}

/// Estremità del client VNC: input verso il server, frame in arrivo.
pub struct CanaleVnc {
    pub input: mpsc::Sender<ComandoVnc>,
    pub frame: mpsc::Receiver<FrameVnc>,
}

/// Apre una connessione VNC e avvia il task (thread) di gestione.
pub fn apri(host: &str, porta: u16, password: Option<String>) -> Result<CanaleVnc, String> {
    let stream = TcpStream::connect((host, porta))
        .map_err(|e| format!("connessione VNC fallita: {e}"))?;

    let pw = password.clone();
    let mut client = Client::from_tcp_stream(stream, true, |metodi| {
        for m in metodi {
            match m {
                AuthMethod::None => return Some(AuthChoice::None),
                AuthMethod::Password => {
                    if let Some(p) = &pw {
                        let mut b = [0u8; 8];
                        for (i, c) in p.bytes().take(8).enumerate() {
                            b[i] = c;
                        }
                        return Some(AuthChoice::Password(b));
                    }
                }
                _ => {}
            }
        }
        None
    })
    .map_err(|e| format!("autenticazione VNC fallita: {e}"))?;

    let (mut larghezza, mut altezza) = client.size();
    let formato = client.format();
    client
        .set_encodings(&[Encoding::Raw, Encoding::DesktopSize])
        .map_err(|e| e.to_string())?;

    let (tx_in, mut rx_in) = mpsc::channel::<ComandoVnc>(128);
    let (tx_frame, rx_frame) = mpsc::channel::<FrameVnc>(64);

    std::thread::spawn(move || {
        // Prima richiesta: schermo intero non incrementale.
        let _ = client.request_update(
            Rect {
                left: 0,
                top: 0,
                width: larghezza,
                height: altezza,
            },
            false,
        );

        loop {
            // Inoltra l'input in attesa.
            let mut chiudi = false;
            while let Ok(cmd) = rx_in.try_recv() {
                match cmd {
                    ComandoVnc::Mouse { x, y, bottoni } => {
                        let _ = client.send_pointer_event(bottoni, x, y);
                    }
                    ComandoVnc::Tasto { giu, key } => {
                        let _ = client.send_key_event(giu, key);
                    }
                    ComandoVnc::Chiudi => chiudi = true,
                }
            }
            if chiudi {
                break;
            }

            // Chiede gli aggiornamenti e li elabora fino a fine frame.
            if client
                .request_update(
                    Rect {
                        left: 0,
                        top: 0,
                        width: larghezza,
                        height: altezza,
                    },
                    true,
                )
                .is_err()
            {
                break;
            }

            let mut fine = false;
            while !fine {
                match client.poll_event() {
                    Some(Event::EndOfFrame) | None => fine = true,
                    Some(Event::PutPixels(r, dati)) => {
                        let rgba = converti(&dati, &formato, r.width, r.height);
                        if tx_frame
                            .blocking_send(FrameVnc {
                                x: r.left,
                                y: r.top,
                                w: r.width,
                                h: r.height,
                                rgba,
                                resize: None,
                            })
                            .is_err()
                        {
                            return;
                        }
                    }
                    Some(Event::Resize(nw, nh)) => {
                        larghezza = nw;
                        altezza = nh;
                        let _ = tx_frame.blocking_send(FrameVnc {
                            x: 0,
                            y: 0,
                            w: nw,
                            h: nh,
                            rgba: Vec::new(),
                            resize: Some((nw, nh)),
                        });
                        let _ = client.request_update(
                            Rect {
                                left: 0,
                                top: 0,
                                width: nw,
                                height: nh,
                            },
                            false,
                        );
                    }
                    Some(Event::Disconnected(_)) => return,
                    Some(_) => {}
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    });

    Ok(CanaleVnc {
        input: tx_in,
        frame: rx_frame,
    })
}

/// Converte un blocco di pixel dal formato del server a RGBA.
fn converti(dati: &[u8], f: &PixelFormat, w: u16, h: u16) -> Vec<u8> {
    let bpp = (f.bits_per_pixel / 8) as usize;
    let n = (w as usize) * (h as usize);
    let mut out = vec![0u8; n * 4];
    for i in 0..n {
        let off = i * bpp.max(1);
        if off + bpp > dati.len() {
            break;
        }
        let px: u32 = match bpp {
            4 => {
                let a = [dati[off], dati[off + 1], dati[off + 2], dati[off + 3]];
                if f.big_endian {
                    u32::from_be_bytes(a)
                } else {
                    u32::from_le_bytes(a)
                }
            }
            2 => {
                let a = [dati[off], dati[off + 1]];
                if f.big_endian {
                    u16::from_be_bytes(a) as u32
                } else {
                    u16::from_le_bytes(a) as u32
                }
            }
            _ => dati[off] as u32,
        };
        out[i * 4] = componente(px, f.red_shift, f.red_max);
        out[i * 4 + 1] = componente(px, f.green_shift, f.green_max);
        out[i * 4 + 2] = componente(px, f.blue_shift, f.blue_max);
        out[i * 4 + 3] = 255;
    }
    out
}

fn componente(px: u32, shift: u8, max: u16) -> u8 {
    if max == 0 {
        return 0;
    }
    let v = (px >> shift) & (max as u32);
    ((v * 255) / (max as u32)) as u8
}

#[cfg(test)]
mod test {
    use super::componente;

    #[test]
    fn estrae_e_scala_il_canale() {
        // Pixel 0xRRGGBB con canali a 8 bit (max 255).
        let px = 0x00_80_40_u32; // r=0x00? no: shift 16 -> 0, 8 -> 0x80, 0 -> 0x40
        assert_eq!(componente(px, 8, 255), 0x80);
        assert_eq!(componente(px, 0, 255), 0x40);
        assert_eq!(componente(px, 16, 255), 0x00);
    }

    #[test]
    fn massimo_e_minimo() {
        assert_eq!(componente(0xFF, 0, 255), 255);
        assert_eq!(componente(0, 0, 255), 0);
    }
}
