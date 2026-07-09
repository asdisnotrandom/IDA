use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio_serial::SerialPortBuilderExt;
use crate::sensorler::m8n::DParse::{HFirst, HSec};
use crate::veri_tipleri::GpsVeri;

const USB_PORT: &str = "asdadas";

enum DParse
{
    HFirst,
    HSec,
}

pub async fn gps_task(tx: mpsc::Sender<GpsVeri>)
{
    let mut usb_port = tokio_serial::new(USB_PORT, 115200).open_native_async().expect("Gps portu acilmadi!");
    let mut buf = [0u8; 1];
    let mut bucket = [0u8; 31];
    let mut duruum = DParse::HFirst;
    loop {
        match duruum
        {
            HFirst =>
            {
                if let Ok(_) = usb_port.read_exact(&mut buf).await
                {
                    if buf[0] == 0xAA
                    {
                        duruum = DParse::HSec;
                    }
                }
            }
            HSec =>
            {
                if let Ok(_) = usb_port.read_exact(&mut buf).await
                {
                    if buf[0] == 0xBB
                    {
                        if usb_port.read_exact(&mut bucket).await.is_ok()
                        {
                            let mut calc_checksum = 0u8;
                            for i in 0..30 { calc_checksum ^= bucket[i]; }
                            if calc_checksum == bucket[30]
                            {
                                let fill_struct = |i: usize| -> i32
                                {
                                    i32::from_le_bytes([bucket[i], bucket[i+1], bucket[i+2], bucket[i+3]].try_into().unwrap())
                                };
                                let paket = GpsVeri {
                                    algi_boyut: bucket[0],
                                    uydu_sayi: bucket[1],
                                    boylam: fill_struct(2),
                                    enlem: fill_struct(6),
                                    yukseklik_mm: fill_struct(10),
                                    hiz: fill_struct(14),
                                    yonelim: fill_struct(18),
                                    zaman_ms: u64::from_le_bytes(bucket[22..30].try_into().unwrap()),

                                };
                                if let Err(e) = tx.send(paket).await
                                {
                                    eprintln!("Gps gonderım hatası: {:?}", e);
                                }
                                duruum = DParse::HFirst;
                            }
                            else {
                                duruum = DParse::HFirst;
                            }
                        }
                        else {
                            duruum = DParse::HFirst;
                        }
                    }
                    else if buf[0] == 0xAA {
                        duruum = DParse::HSec;
                    }
                    else {
                        duruum = DParse::HFirst;
                    }
                }
            }
        }
    }
}