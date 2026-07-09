mod sensorler;
mod motorlar;
mod veri_tipleri;
mod beyin;

use tokio::sync::{mpsc, watch};
use veri_tipleri::{ImuVeri};

use crate::veri_tipleri::{GpsVeri, LidarVeri, MotorVeri};

#[tokio::main]
async fn main()
{
    let (imu_tx, mut imu_rx) = mpsc::channel::<ImuVeri>(100);
    let (gps_tx, gps_rx) = mpsc::channel::<GpsVeri>(100);
    let (lid_tx, lid_rx) = mpsc::channel::<LidarVeri>(100);

    tokio::spawn(async move {
        sensorler::m8n::gps_task(gps_tx).await;
    });
    tokio::spawn(async move {
        sensorler::bno085::imu_task(imu_tx).await;
    });
    tokio::spawn(async move {
        sensorler::rplidars3::lidar_task(lid_tx).await;
    });
    let imu_paket = imu_rx.recv().await;
}