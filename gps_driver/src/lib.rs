#![no_std]

use embedded_io_async::Write;
use embedded_io_async::{BufRead, ErrorType};
use embassy_time;

use crate::UbxAsama::PayloadPart;
#[derive(Debug, Default)]
pub struct GPSOutput {
    pub algi_boyut: u8,
    pub uydu_sayi: u8,
    pub boylam: i32,
    pub enlem: i32,
    pub yukseklik_mm: i32,
    pub hiz: i32,
    pub yonelim: i32,
}

#[derive(Debug)]
pub enum UbxError<E> {
    Uart(E),
    CheksumErr,
    BuffOver,
    MIDErr,
}

#[derive(Debug, PartialEq)]
enum UbxAsama {
    Sync1Part,
    Sync2Part,
    ClassPart,
    IDPart,
    Len1Part,
    Len2Part,
    PayloadPart,
    Cheksum1Part,
    Cheksum2Part,
}

pub struct UbloxSablon<UART> {
    uart: UART,
    asama: UbxAsama,
    msg_class: u8,
    msg_id: u8,
    payload_len: u16,
    payload_idx: usize,
    payload_buffer: [u8; 100],
    check1: u8,
    check2: u8,
}

impl<UART> UbloxSablon<UART>
where
    UART: BufRead + ErrorType,
{
    pub fn new(uart: UART) -> Self {
        Self {
            uart,
            asama: UbxAsama::Sync1Part,
            msg_class: 0,
            msg_id: 0,
            payload_len: 0,
            payload_idx: 0,
            payload_buffer: [0u8; 100],
            check1: 0,
            check2: 0,
        }
    }
    pub fn checkup(chec1: &mut u8, chec2: &mut u8, byte: u8) {
        *chec1 = chec1.wrapping_add(byte);
        *chec2 = chec2.wrapping_add(*chec1);
    }
    fn parse_bidi(buff: &[u8; 100]) -> GPSOutput {
        let i32_conv = |idx: usize| -> i32 {
            i32::from_le_bytes([buff[idx], buff[idx + 1], buff[idx + 2], buff[idx + 3]])
        };
        GPSOutput {
            algi_boyut: buff[20],
            uydu_sayi: buff[23],
            boylam: i32_conv(24),
            enlem: i32_conv(28),
            yukseklik_mm: i32_conv(32),
            hiz: i32_conv(60),
            yonelim: i32_conv(64),
        }
    }
    pub async fn aktif_et_nav_pvt<W>(tx: &mut W) -> Result<(), W::Error>
    where
        W: Write,
    {
        const ENABLE_NAV_PVT: [u8; 11] = [
            0xB5, 0x62, 0x06, 0x01, 0x03, 0x00, 0x01, 0x07, 0x01, 0x13, 0x51,
        ];
        tx.write_all(&ENABLE_NAV_PVT).await?;
        embassy_time::Timer::after(embassy_time::Duration::from_millis(20)).await;
        const CFG_RATE_10HZ: [u8; 14] = [
            0xB5, 0x62, // Sync
            0x06, 0x08, // Class (CFG) ve ID (RATE)
            0x06, 0x00, // Payload Length (6 byte)
            0x64, 0x00, // measRate: 100 ms (10Hz)
            0x01, 0x00, // navRate: 1
            0x01, 0x00, // timeRef: 1 (GPS time)
            0x7A, 0x12, // Checksum
        ];
        tx.write_all(&CFG_RATE_10HZ).await?;
        Ok(())
    }
    pub async fn read_data(&mut self) -> Result<GPSOutput, UbxError<UART::Error>> {
        loop {
            let buffer = self.uart.fill_buf().await.map_err(UbxError::Uart)?;
            let mut kullan_byte = 0;
            let mut sonuc = None;
            for &b in buffer {
                kullan_byte += 1;
                match self.asama {
                    UbxAsama::Sync1Part => {
                        if b == 0xB5 {
                            self.asama = UbxAsama::Sync2Part;
                        }
                    }
                    UbxAsama::Sync2Part => {
                        if b == 0x62 {
                            self.asama = UbxAsama::ClassPart;
                            self.check1 = 0;
                            self.check2 = 0;
                        } else {
                            self.asama = UbxAsama::Sync1Part;
                        }
                    }
                    UbxAsama::ClassPart => {
                        Self::checkup(&mut self.check1, &mut self.check2, b);
                        self.msg_class = b;
                        self.asama = UbxAsama::IDPart;
                    }
                    UbxAsama::IDPart => {
                        Self::checkup(&mut self.check1, &mut self.check2, b);
                        self.msg_id = b;
                        self.asama = UbxAsama::Len1Part;
                    }
                    UbxAsama::Len1Part => {
                        Self::checkup(&mut self.check1, &mut self.check2, b);
                        self.payload_len = b as u16;
                        self.asama = UbxAsama::Len2Part;
                    }
                    UbxAsama::Len2Part => {
                        Self::checkup(&mut self.check1, &mut self.check2, b);
                        self.payload_len |= (b as u16) << 8;
                        self.payload_idx = 0;
                        if self.payload_len > self.payload_buffer.len() as u16 {
                            self.asama = UbxAsama::Sync1Part;
                            sonuc = Some(Err(UbxError::BuffOver));
                            break;
                        }
                        if self.payload_len == 0
                        {
                            self.asama = UbxAsama::Cheksum1Part;
                        }
                        else {
                            self.asama = PayloadPart;
                        }
                    }
                    UbxAsama::PayloadPart => {
                        Self::checkup(&mut self.check1, &mut self.check2, b);
                        self.payload_buffer[self.payload_idx] = b;
                        self.payload_idx += 1;
                        if self.payload_idx == self.payload_len as usize {
                            self.asama = UbxAsama::Cheksum1Part;
                        }
                    }
                    UbxAsama::Cheksum1Part => {
                        if b != self.check1 {
                            self.asama = UbxAsama::Sync1Part;
                            sonuc = Some(Err(UbxError::CheksumErr));
                            break;
                        }
                        self.asama = UbxAsama::Cheksum2Part;
                    }
                    UbxAsama::Cheksum2Part => {
                        self.asama = UbxAsama::Sync1Part;
                        if b != self.check2 {
                            sonuc = Some(Err(UbxError::CheksumErr));
                            break;
                        }
                        if self.msg_id == 0x07 && self.msg_class == 0x01 {
                            sonuc = Some(Ok(Self::parse_bidi(&self.payload_buffer)));
                            break;
                        } else {
                            
                        }
                    }
                }
            }
            self.uart.consume(kullan_byte);
            if let Some(sonuc_son) = sonuc {
                return sonuc_son;
            }
        }
    }
}
