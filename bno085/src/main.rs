#![no_std]
#![no_main]

use core::f32::consts::PI;
use libm::{atan2f, asinf};
use embassy_rp::gpio::Input;
use embassy_rp::i2c::{Async, I2c};
use embassy_rp::{bind_interrupts};
use embassy_rp::peripherals::{USB};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_executor::Spawner;
use embassy_usb::Config;
use embassy_usb::Builder;
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::class::cdc_acm::State;
use embassy_time::{Timer, Instant};
use static_cell::StaticCell;
use panic_probe as _;
bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
});

fn telemetrry_pack(quat: (f32,f32,f32), gyro: (f32,f32,f32), accel: (f32,f32,f32), zaman_ms: u64) -> [u8; 47]
{
    let mut pack = [0u8; 47];
    pack[0] = 0xAA;
    pack[1] = 0xBB;
    let mut offset = 2;
    for &val in &[quat.0, quat.1, quat.2, gyro.0,gyro.1,gyro.2,accel.0,accel.1,accel.2]
    {
        pack[offset..offset+4].copy_from_slice(&val.to_le_bytes());
        offset+=4;
    }
    pack[offset..offset+8].copy_from_slice(&zaman_ms.to_le_bytes());
    let mut checksum = 0u8;
    for i in 2..46
    {
        checksum ^= pack[i];
    }
    pack[46] = checksum;
    pack
}
fn quat_to_euler(r: f32, i: f32, j: f32, k: f32) -> (f32, f32, f32)
{
//roll
let sinr_cosp = 2.0 * (r * i + j * k);
let cosr_cosp = 1.0 - 2.0 * (i * i + j * j);
let roll = atan2f(sinr_cosp, cosr_cosp);
//pitch
let sinp = (2.0 * (r * j - k * i)).clamp(-1.0,1.0);
let pitch = asinf(sinp);
//yaw
let siny_cosp = 2.0 * (r * k + i * j);
let cosy_cosp = 1.0 - 2.0 * (j * j + k * k);
let yaw = atan2f(siny_cosp, cosy_cosp);
//derece
let to_deg = 180.0 / PI;
(roll * to_deg, pitch * to_deg, yaw * to_deg)
}

#[embassy_executor::task]
async fn usb_task(mut dev: UsbDevice<'static, Driver<'static, USB>>)
{
    dev.run().await;
}

#[embassy_executor::task]
async fn imu_task(i2c_bus: I2c<'static, embassy_rp::peripherals::I2C0, Async>, hint_pin: embassy_rp::gpio::Input<'static>, mut usb_tx: CdcAcmClass<'static, Driver<'static, USB>>)
{
    Timer::after_secs(5).await;
    let mut sensor = bno085::Bno085Imu::new(i2c_bus, hint_pin, 0x4A);
    sensor.active_fut(0x05, 20_000).await.unwrap();
    Timer::after_millis(50).await;
    sensor.active_fut(0x02, 20_000).await.unwrap();
    Timer::after_millis(50).await;
    sensor.active_fut(0x04, 20_000).await.unwrap();
    Timer::after_millis(50).await;
    let mut last_quat = (0.0, 0.0, 0.0);
    let mut last_gyro = (0.0, 0.0, 0.0);
    let mut last_accel = (0.0, 0.0, 0.0);
    loop {
        match sensor.read_val().await
        {
            Ok(data) => //usb loglari icin karmasik, tam uarta geciste nefes alacak (if let Some(g) = data.gyro {last_gyro = g}
            {
                if let Some((r,i,j,k)) = data.quat
                {
                    let (roll,pitch,yaw) = quat_to_euler(r, i, j, k);
                    last_quat = quat_to_euler(r, i, j, k);
                    log::info!("Quaternion -> Roll: {:>6.1} | Pitch: {:>6.1} | Yaw: {:>6.1}", roll,pitch,yaw);
                }
                if let Some((gx,gy,gz)) = data.gyro
                {
                    last_gyro = (gx,gy,gz);
                    log::info!("Gyro -> X: {:>6.2} | Y: {:>6.2} | Z: {:>6.2} rad/s",gx,gy,gz);
                }
                if let Some((ax, ay, az)) = data.accel {
                    last_accel = (ax,ay,az);
                    log::info!("Accel -> X: {:>6.2} | Y: {:>6.2} | Z: {:>6.2} m/s^2", ax, ay, az);
                }
                let milisaniye: u64 = Instant::now().as_millis() as u64;
                let binary_pack = telemetrry_pack(last_quat, last_gyro, last_accel, milisaniye);
                if let Err(_) = usb_tx.write_packet(&binary_pack).await
                {
                    log::error!("DMA Uart hatasi");
                }
            }
            Err(e) =>
            {
                log::error!("Okuma hatasi: {:?}", e);
                Timer::after_millis(50).await;
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner)
{
    let p = embassy_rp::init(Default::default());
    let usb_driver = Driver::new(p.USB, Irqs);
    let i2c = I2c::new_async(
        p.I2C0,
        p.PIN_5,
        p.PIN_4,
        Irqs,
        Default::default(),
    );
    let usb_conf = Config::new(0x1029, 0x0001);
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static STATE: StaticCell<State> = StaticCell::new();
    
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);
    let usb_state = STATE.init(State::new());
    let mut usb_builder = Builder::new(usb_driver, usb_conf, config_descriptor, bos_descriptor, &mut [], control_buf);
    let usb_baba = CdcAcmClass::new(&mut usb_builder, usb_state, 64);
    let usb_device = usb_builder.build();
    let hint = Input::new(p.PIN_16, embassy_rp::gpio::Pull::Up);
    spawner.spawn(usb_task(usb_device).unwrap());
    spawner.spawn(imu_task(i2c, hint, usb_baba).expect("imu task baslamadi!"));
}
