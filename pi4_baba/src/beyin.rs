use tokio::sync::{mpsc, watch};
use crate::veri_tipleri::{GpsVeri,ImuVeri,LidarVeri, MotorVeri};

pub async fn nav_task (
    mut imu_rx: mpsc::Receiver<ImuVeri>,
    mut gps_rx: mpsc::Receiver<GpsVeri>,
    mut lidar_tx: mpsc::Receiver<LidarVeri>,
    motor_tx: watch::Sender<MotorVeri>,
    )
{
    
}