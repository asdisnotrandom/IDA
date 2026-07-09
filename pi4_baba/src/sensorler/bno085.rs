use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio_serial::SerialPortBuilderExt;
use crate::sensorler::bno085::DurumParse::{HeaderF, HeaderS};
use crate::veri_tipleri::ImuVeri;

const USB_PORT: &str = "asdas";

enum DurumParse
{
    HeaderF,
    HeaderS,
}

pub async fn imu_task(tx: mpsc::Sender<ImuVeri>)
{
    let mut usb_port = tokio_serial::new(USB_PORT, 115200).open_native_async().expect("Imu portu acilmadi!");
    let mut buf = [0u8; 1];
    let mut bucket = [0u8; 45];
    let mut durum = DurumParse::HeaderF;
    loop {
        match durum
        {
            HeaderF =>
            {
                if let Ok(_) = usb_port.read_exact(&mut buf).await
                {
                    if buf[0] == 0xAA
                    {
                        durum = DurumParse::HeaderS;
                    }
                }
            }
            HeaderS =>
            {
                if let Ok(_) = usb_port.read_exact(&mut buf).await
                {
                    if buf[0] == 0xBB
                    {
                        if usb_port.read_exact(&mut bucket).await.is_ok()
                        {
                            let mut calc_checksum = 0u8;
                            for i in 0..44 { calc_checksum ^= bucket[i]; }
                            if calc_checksum == bucket[44]
                            {
                                let fill_struct = |i: usize| -> f32
                                {
                                    f32::from_le_bytes([bucket[i], bucket[i+1], bucket[i+2], bucket[i+3]].try_into().unwrap())
                                };
                                let paket = ImuVeri {
                                    roll: fill_struct(0),
                                    pitch: fill_struct(4),
                                    yaw: fill_struct(8),
                                    gx: fill_struct(12),
                                    gy: fill_struct(16),
                                    gz: fill_struct(20),
                                    ax: fill_struct(24),
                                    ay: fill_struct(28),
                                    az: fill_struct(32),
                                    zaman_ms: u64::from_le_bytes(bucket[36..44].try_into().unwrap()),
                                };
                                if let Err(e) = tx.send(paket).await
                                {
                                    eprintln!("Imu gonderım hatası: {:?}", e);
                                }
                                durum = DurumParse::HeaderF;
                            }
                            else {
                                durum = DurumParse::HeaderF;
                            }
                        }
                        else {
                            durum = DurumParse::HeaderF;
                        }
                    }
                    else if buf[0] == 0xAA {
                        durum = DurumParse::HeaderS;
                    }
                    else {
                        durum = DurumParse::HeaderF;
                    }
                }
            }
        }
    }
}