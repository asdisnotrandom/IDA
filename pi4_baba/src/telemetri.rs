use tokio_serial::SerialPortBuilderExt;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::veri_tipleri::*;
use std::{error::Error, time::Duration};
use tokio::sync::mpsc;

fn parse_yk_komut(payload: &str) -> Option<GelenTelemetri> {
    if let Some(data) = payload.strip_prefix("CMD:") {
        let mut parts = data.split(':');
        let command_type = parts.next()?;
        let args = parts.next();

        return match command_type {
            "START" => Some(GelenTelemetri::GoreviBaslat),
            "STOP" => Some(GelenTelemetri::AcilDurdur),
            "MOD" => {
                if let Some(arg_str) = args {
                    if let Ok(mod_id) = arg_str.parse::<u8>() {
                        return Some(GelenTelemetri::ModDegistir(AracMod::from_u8(mod_id)));
                    }
                }
                None
            }
            "MAN" => {
                if let Some(arg_str) = args {
                    let mut vals = arg_str.split(',');
                    if let (Some(gaz_str), Some(aci_str)) = (vals.next(), vals.next()) {
                        if let (Ok(gaz), Ok(aci)) = (gaz_str.parse::<f32>(), aci_str.parse::<f32>()) {
                            return Some(GelenTelemetri::ManuelKontrol(gaz, aci));
                        }
                    }
                }
                None
            }
            "ROTA" => {
                let mut noktalar = Vec::new();
                if let Some(rota_str) = args {
                    for nokta_str in rota_str.split(';') {
                        let mut koordinatlar = nokta_str.split(',');
                        if let (Some(enlem), Some(boylam)) = (koordinatlar.next(), koordinatlar.next()) {
                            if let (Ok(lat), Ok(lon)) = (enlem.parse::<f64>(), boylam.parse::<f64>()) {
                                noktalar.push((lat, lon));
                            }
                        }
                    }
                    if !noktalar.is_empty() {
                        return Some(GelenTelemetri::RotaBelirle(noktalar));
                    }
                }
                None
            }
            _ => None,
        };
    }
    None
}

pub async fn telemetri_task(
    port_adi: &str,
    baud_rate: u32,
    tx_yki: mpsc::Sender<GelenTelemetri>,
    mut rx_yki: mpsc::Receiver<GidenTelemetri>,
    ) -> Result<(), Box<dyn Error>>
{
    let mut tel_port = tokio_serial::new(port_adi, baud_rate).open_native_async()?;
    let (okur, mut yazar) = tokio::io::split(tel_port);
    let mut buf_reader = BufReader::new(okur);
    let rx_task = tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match buf_reader.read_line(&mut line).await {
                Ok(0) => {
                    eprintln!("Telemetri ile baglanti kesildi.");
                    break;
                }
                Ok(_) => {
                    let temiz = line.trim();
                    if let Some((payload, cs)) = temiz.split_once('*') {
                        if calc_checksum(payload) == cs.to_uppercase() {
                            if let Some(komut) = parse_yk_komut(payload) {
                                let _ = tx_yki.send(komut).await;
                            }
                        } else {
                            eprintln!("Hatali checksum: {}", temiz);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Serial port okunamadi: {}", e);
                    break;
                }
            }
        }
    });
    let tx_task = tokio::spawn(async move {
        while let Some(telemetri) = rx_yki.recv().await {
            let (nav_str, mot_str) = telemetri.to_rf_strings();
            
            let _ = yazar.write_all(nav_str.as_bytes()).await;
            let _ = yazar.write_all(mot_str.as_bytes()).await;
        }
    });
    let _ = tokio::try_join!(rx_task, tx_task);
    Ok(())
}

