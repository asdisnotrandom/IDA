#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_stm32::gpio::OutputType::PushPull;
use embedded_io_async::Read;
use ida_motor_stm::MotorData;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::peripherals::{USART1};
use embassy_stm32::timer::simple_pwm::PwmPin;
use embassy_stm32::timer::{Ch1, Channel};
use embassy_stm32::{bind_interrupts};
use embassy_stm32::usart::{BufferedInterruptHandler, BufferedUart, Config as UConfig};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use static_cell::StaticCell;

bind_interrupts!(struct Irqs
{
    USART1 => BufferedInterruptHandler<USART1>;
});
static PWM_VALUES: Signal<CriticalSectionRawMutex, [u16; 4]> = Signal::new();

fn compute_crc8(data: &[u8]) -> u8 {
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

#[embassy_executor::task]
async fn uart_task(mut rx: BufferedUart<'static>)
{
    let mut bucket = [0u8; 11];
    loop {
        let mut tek_byte = [0u8; 1];
        if rx.read_exact(&mut tek_byte).await.is_err() { continue; }
        if tek_byte[0] != 0xAA { continue; }
        bucket[0] = 0xAA;
        if rx.read_exact(&mut tek_byte).await.is_err() { continue; }
        if tek_byte[0] != 0x55 { continue; }
        bucket[1] = 0x55;
        if rx.read_exact(&mut bucket[2..11]).await.is_err()
        {
            warn!("okuma hatasi(paket yarim)");
            continue;
        }
        if compute_crc8(&bucket[0..10]) == bucket[10]
        {
            let pwm1 = ((bucket[2] as u16) << 8) | (bucket[3] as u16);
            let pwm2 = ((bucket[4] as u16) << 8) | (bucket[5] as u16);
            let pwm3 = ((bucket[6] as u16) << 8) | (bucket[7] as u16);
            let pwm4 = ((bucket[8] as u16) << 8) | (bucket[9] as u16);
            PWM_VALUES.signal([pwm1, pwm2, pwm3, pwm4]);
        }
        else {
            warn!("CRC Hatali, paket cop")
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner)
{
    let p = embassy_stm32::init(Default::default());
    static TX_BUF: StaticCell<[u8; 100]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 100])[..];
    static RX_BUF: StaticCell<[u8; 100]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 100])[..];

    let mut uart_config = UConfig::default();
    uart_config.baudrate = 115200;
    let buart = BufferedUart::new(p.USART1, p.PA10, p.PA9, tx_buf, rx_buf, Irqs, uart_config).unwrap();
    let ch1_pin: PwmPin<'_, embassy_stm32::peripherals::TIM2, Ch1, embassy_stm32::gpio::AfioRemap<0>> = PwmPin::new(p.PA0, PushPull);
    let ch2_pin = PwmPin::new(p.PA1, PushPull);
    let ch3_pin = PwmPin::new(p.PA2, PushPull);
    let ch4_pin = PwmPin::new(p.PA3, PushPull);

    let mut pwm = embassy_stm32::timer::simple_pwm::SimplePwm::new(
        p.TIM2,
        Some(ch1_pin),
        Some(ch2_pin),
        Some(ch3_pin),
        Some(ch4_pin),
        embassy_stm32::time::hz(50),
        Default::default(),
    );
    pwm.enable(Channel::Ch1);
    pwm.enable(Channel::Ch2);
    pwm.enable(Channel::Ch3);
    pwm.enable(Channel::Ch4);
    let max_duty_val: u16 = pwm.max_duty_cycle() as u16;
    let kanallar = pwm.split();
    let mut iskeleon = MotorData::new(kanallar.ch1, max_duty_val);
    let mut iskelearka = MotorData::new(kanallar.ch2, max_duty_val);
    let mut sancakon = MotorData::new(kanallar.ch3, max_duty_val);
    let mut sancakarka = MotorData::new(kanallar.ch4, max_duty_val);
    spawner.spawn(uart_task(buart).unwrap());
    let mut aktif_pwms = [0u16; 4];
    let _ = iskeleon.esc_baslat();
    let _ = iskelearka.esc_baslat();
    let _ = sancakon.esc_baslat();
    let _ = sancakarka.esc_baslat();
    embassy_time::Timer::after_secs(2).await; // motorların çalışması kaç saniye ya da milisaniyeyse düşür. 
    loop {
        if PWM_VALUES.signaled()
        {
            aktif_pwms = PWM_VALUES.wait().await;
        }
        let _ = iskeleon.set_speed(aktif_pwms[0]);
        let _ = iskelearka.set_speed(aktif_pwms[1]);
        let _ = sancakon.set_speed(aktif_pwms[2]);
        let _ = sancakarka.set_speed(aktif_pwms[3]);
        embassy_time::Timer::after_millis(20).await;
    }
}
