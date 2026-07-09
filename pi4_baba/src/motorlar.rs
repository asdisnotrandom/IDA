use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio::io::AsyncWriteExt;

pub struct MotorKontrol
{
    port: SerialStream,
}

impl MotorKontrol
{
    pub fn new_port(port_name: &str) -> tokio_serial::Result<Self>
    {
        let port = tokio_serial::new(port_name, 115_200).open_native_async()?;
        Ok(Self { port })
    }
    fn calc_crc8(data: &[u8]) -> u8 {
        let mut crc = 0x00;
        for &byte in data {
            crc ^= byte;
            for _ in 0..8 {
                if (crc & 0x80) != 0 {
                    crc = crc.wrapping_shl(1) ^ 0x8C;
                } else {
                    crc = crc.wrapping_shl(1);
                }
            }
        }
        crc
    }
    pub async fn set_speeds(&mut self, iskeleon: u16, iskelearka: u16, sancakon: u16, sancakarka: u16) -> std::io::Result<()>
    {
        let io = iskeleon.clamp(0,1000);
        let ia = iskelearka.clamp(0, 1000);
        let so = sancakon.clamp(0, 1000);
        let sa = sancakarka.clamp(0, 1000);
        let mut bucket = [0u8; 11];
        bucket[0] = 0xAA;
        bucket[1] = 0x55;
        bucket[2] = (io >> 8) as u8;
        bucket[3] = (io & 0xFF) as u8;
        bucket[4] = (ia >> 8) as u8;
        bucket[5] = (ia & 0xFF) as u8;
        bucket[6] = (so >> 8) as u8;
        bucket[7] = (so & 0xFF) as u8;
        bucket[8] = (sa >> 8) as u8;
        bucket[9] = (sa & 0xFF) as u8;
        bucket[10] = Self::calc_crc8(&bucket[0..10]);
        self.port.write_all(&bucket).await?;
        self.port.flush().await?;
        Ok(())
    }
}