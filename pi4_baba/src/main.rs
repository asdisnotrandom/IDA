mod sensorler;
mod motorlar;
mod veri_tipleri;
mod beyin;

use tokio::sync::{mpsc, watch};
use veri_tipleri::{ImuVeri};

use crate::veri_tipleri::{GpsVeri, LidarVeri, MotorVeri};
use crate::motorlar::MotorKontrol;
#[tokio::main]
async fn main()
{
    let (imu_tx, imu_rx) = watch::channel(ImuVeri::default());
    let (gps_tx, gps_rx) = watch::channel(GpsVeri::default());
    let (motor_tx, mut motor_rx) = mpsc::channel::<MotorVeri>(100);

    tokio::spawn(async move {
        sensorler::m8n::gps_task(gps_tx).await;
    });
    tokio::spawn(async move {
        sensorler::bno085::imu_task(imu_tx).await;
    });
    //tokio::spawn(async move {
    //    sensorler::rplidars3::lidar_task(lid_tx).await;
    //});
    tokio::spawn(async move {
        let mut motor_kontrol = match MotorKontrol::new_port("/dev/ttyUSB0") {
            Ok(mk) => mk,
            Err(e) => {
                eprintln!("Motor portu acilmadi: {:?}", e);
                return;
            }
        };
        while let Some(motor_komutu) = motor_rx.recv().await {
            let sonuc = motor_kontrol.set_speeds(
                motor_komutu.iskeleon,
                motor_komutu.iskelearka,
                motor_komutu.sancakon,
                motor_komutu.sancakarka
            ).await;
            if let Err(e) = sonuc {
                eprintln!("Motorlara komut gonderilemedi: {:?}", e);
            }
        }
    });
    let nav_handle = tokio::spawn(async move {
        beyin::nav_task(imu_rx, gps_rx, motor_tx).await;
    });
    let _ = nav_handle.await;
}