#![no_std]

use embedded_hal_async::digital::Wait;
use embedded_hal_async::i2c::I2c;
use core::result::Result::{Err,Ok};
use core::derive;
use core::fmt::Debug;
#[derive(Debug)]
pub enum BnoError<I2cE>
{
    I2c(I2cE),
    InvalidLength(u16),
    WrongChannel(u8),
    UnexpectedReport(u8),
    PinError,
}

pub struct Bno085Imu<I2C, HINT>
{
    i2c: I2C,
    hint_pin: HINT,
    address: u8,
}

#[derive(Debug, Default)]
pub struct BnoData
{
    pub quat: Option<(f32,f32,f32,f32)>, //r, i ,j,k olarak dnecek
    pub gyro: Option<(f32,f32,f32)>, // rad/s biriminde x,y,z
    pub accel: Option<(f32,f32,f32)>, // m/s^2 biriminde x,y,z
}

impl<I2C, HINT, I2cE> Bno085Imu<I2C, HINT>
where
    I2C: I2c<Error = I2cE>,
    HINT: Wait,
{
    pub fn new(i2c: I2C, hint_pin: HINT, address: u8) -> Self
    {
        Self { i2c, hint_pin, address }
    }
    pub async fn active_fut(&mut self, fut_id: u8, timehz: u32) -> Result<(), BnoError<I2cE>>
    {
        let mut send_pack = [0u8; 21];
        send_pack[0] = 21;
        send_pack[1] = 0;
        send_pack[2] = 2;
        send_pack[3] = 0;
        send_pack[4] = 0xFD;
        send_pack[5] = fut_id;
        send_pack[9] = (timehz & 0xFF) as u8;
        send_pack[10] = ((timehz >> 8) & 0xFF) as u8;
        send_pack[11] = ((timehz >> 16) & 0xFF) as u8;
        send_pack[12] = ((timehz >> 24) & 0xFF) as u8;
        self.i2c.write(self.address, &send_pack).await.map_err(BnoError::I2c)?;
        Ok(())
    }
    pub async fn read_val(&mut self) -> Result<BnoData, BnoError<I2cE>>
    {
        self.hint_pin.wait_for_low().await.map_err(|_| BnoError::PinError)?;
        let mut header_buff = [0u8; 4];
        self.i2c.read(self.address, &mut header_buff).await.map_err(BnoError::I2c)?;
        let bucket_length = ((header_buff[1] as u16) << 8 | (header_buff[0] as u16)) & 0x7FFF;
        let channel = header_buff[2];
        if bucket_length <= 4
        {
            return Err(BnoError::InvalidLength(bucket_length));
        }
        let mut buffer = [0u8; 300];
        let read_len = bucket_length as usize;
        if read_len > buffer.len()
        {
            return Err(BnoError::InvalidLength(bucket_length));
        }
        self.i2c.read(self.address, &mut buffer[..read_len]).await.map_err(BnoError::I2c)?;
        let mut data = BnoData::default();
        let mut has_data = false;
        let mut unknown_id = 0;
        if channel == 3
        {
            let mut index = 4;
            while index < read_len
            {
                let report_id = buffer[index];
                if report_id == 0x05 && index + 14 <= read_len
                {
                    //quaternion
                    let i = i16::from_le_bytes([buffer[index + 4], buffer[index + 5]]);
                    let j = i16::from_le_bytes([buffer[index + 6], buffer[index + 7]]);
                    let k = i16::from_le_bytes([buffer[index + 8], buffer[index + 9]]);
                    let r = i16::from_le_bytes([buffer[index + 10], buffer[index + 11]]);
                    data.quat = Some((r as f32 / 16384.0, i as f32 / 16384.0, j as f32 / 16384.0, k as f32 / 16384.0));
                    index += 14;
                    has_data = true;
                }
                else if report_id == 0x02 && index + 10 <= read_len
                {
                    //jiroskop
                    let x = i16::from_le_bytes([buffer[index + 4], buffer[index + 5]]);
                    let y = i16::from_le_bytes([buffer[index + 6], buffer[index + 7]]);
                    let z = i16::from_le_bytes([buffer[index + 8], buffer[index + 9]]);
                    data.gyro = Some((x as f32 / 512.0, y as f32 / 512.0, z as f32 / 512.0));
                    index += 10;
                    has_data = true;
                }
                else if report_id == 0x04 && index + 10 <= read_len
                {
                    //doğrusal ivme
                    let x = i16::from_le_bytes([buffer[index + 4], buffer[index + 5]]);
                    let y = i16::from_le_bytes([buffer[index + 6], buffer[index + 7]]);
                    let z = i16::from_le_bytes([buffer[index + 8], buffer[index + 9]]);
                    data.accel = Some((x as f32 / 256.0, y as f32 / 256.0, z as f32 / 256.0));
                    index += 10;
                    has_data = true;
                }
                else if report_id == 0xFB && index + 5 <= read_len
                {
                    //gereksiz zaman dalgalarına karşı atlama
                    index += 5;
                }
                else
                {
                    //yanlış, eksik raporda döngüden çık, üzme
                    unknown_id = report_id;
                    break;
                }
            }
        }
        else
        {
            return Err(BnoError::WrongChannel(channel));
        }
        if has_data
        {
            Ok(data)
        }
        else
        {
            Err(BnoError::UnexpectedReport(unknown_id)) // verisiz gereksiz paket
        }
    }
}