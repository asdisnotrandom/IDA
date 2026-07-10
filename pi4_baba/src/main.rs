mod sensorler;
mod motorlar;
mod veri_tipleri;
mod beyin;
mod telemetri;

use tokio::sync::{mpsc, watch};
use veri_tipleri::ImuVeri;
use crate::veri_tipleri::{GpsVeri, LidarVeri, MotorVeri, GelenTelemetri, GidenTelemetri};
use crate::motorlar::MotorKontrol;

const TEL_BAUD_RATE: u32 = 57600;
const TEL_PORT_AD: &str = "hoppa";
const MOTOR_PORT: &str = "bobba";

#[tokio::main]
async fn main() {
    const TEL_CHANNEL_BUF: usize = 100;
    const MOTOR_CHANNEL_BUF: usize = 100;
    let (tel_to_beyin_tx, tel_to_beyin_rx) = mpsc::channel::<GelenTelemetri>(TEL_CHANNEL_BUF);
    let (beyin_to_tel_tx, beyin_to_tel_rx) = mpsc::channel::<GidenTelemetri>(TEL_CHANNEL_BUF);
    let (imu_tx, imu_rx) = watch::channel(ImuVeri::default());
    let (gps_tx, gps_rx) = watch::channel(GpsVeri::default());
    let (motor_tx, mut motor_rx) = mpsc::channel::<MotorVeri>(MOTOR_CHANNEL_BUF);
    {
        let gps_tx = gps_tx.clone();
        let gps_handle = tokio::spawn(async move {
            sensorler::m8n::gps_task(gps_tx).await;
        });
    }
    {
        let imu_tx = imu_tx.clone();
        let imu_handle = tokio::spawn(async move {
            sensorler::bno085::imu_task(imu_tx).await;
        });
    }
    let motor_handle = tokio::spawn(async move {
        let mut motor_kontrol = match MotorKontrol::new_port(MOTOR_PORT)
        {
            Ok(mk) => mk,
            Err(e) =>
            {
                eprintln!("Motor portu acilmadi: {:?}", e);
                return;
            }
        };

        while let Some(motor_komutu) = motor_rx.recv().await
        {
            let sonuc = motor_kontrol.set_speeds(
                motor_komutu.iskeleon,
                motor_komutu.iskelearka,
                motor_komutu.sancakon,
                motor_komutu.sancakarka
            ).await;
            if let Err(e) = sonuc
            {
                eprintln!("Motorlara komut gonderilemedi: {:?}", e);
            }
        }
    });
    let tel_port = TEL_PORT_AD.to_string();
    let tel_baud = TEL_BAUD_RATE;
    let tel_handle =
    {
        let tx_yki = tel_to_beyin_tx.clone();
        let rx_yki = beyin_to_tel_rx;
        tokio::spawn(async move {
            if let Err(e) = telemetri::telemetri_task(&tel_port, tel_baud, tx_yki, rx_yki).await {
                eprintln!("Telemetri görev hatası: {:?}", e);
            }
        })
    };
    let nav_handle = tokio::spawn(async move {
        beyin::nav_task(imu_rx, gps_rx, motor_tx, tel_to_beyin_rx, beyin_to_tel_tx).await;
    });
    tokio::select! {
    res = nav_handle => {
        println!("nav_task sonlandı: {:?}", res);
    }
    res = motor_handle => {
        eprintln!("MOTORLAR DUDRU: {:?}", res);
    }
    res = tel_handle => {
        println!("Telemetri sonlandı: {:?}", res);
    }
}
println!("Sistem kapanıyor.");
}