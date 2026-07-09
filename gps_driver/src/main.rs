#![no_std]
#![no_main]

use gps_driver::UbloxSablon;
use panic_probe as _;
use embassy_usb::{Config, Builder, UsbDevice};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_rp::peripherals::{UART0, USB};
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::bind_interrupts;
use embassy_executor::Spawner;
use embassy_rp::uart::{BufferedInterruptHandler, BufferedUart, BufferedUartRx, BufferedUartTx, Config as UConfig};
use static_cell::StaticCell;
bind_interrupts!(struct Irqs
{
    USBCTRL_IRQ => InterruptHandler<USB>;
    UART0_IRQ => BufferedInterruptHandler<UART0>;
});

#[embassy_executor::task]
async fn usb_task(mut dev: UsbDevice<'static, Driver<'static, USB>>)
{
    dev.run().await;
}

#[embassy_executor::task]
async fn gps_task(mut tx: BufferedUartTx, rx: BufferedUartRx, mut usb_tx: CdcAcmClass<'static, Driver<'static, USB>>)
{
    let mut sablon = UbloxSablon::new(rx);
    if let Err(_) = UbloxSablon::<BufferedUartRx>::aktif_et_nav_pvt(&mut tx).await
    {
        log::error!("gps aktif edilemedi!");
    }
    loop
    {
        match sablon.read_data().await
        {
            Ok(data) =>
            {
                let mut bucket = [0u8; 33];
                bucket[0] = 0xAA;
                bucket[1] = 0xBB;
                bucket[2] = data.algi_boyut;
                bucket[3] = data.uydu_sayi;
                bucket[4..8].copy_from_slice(&data.boylam.to_le_bytes());
                bucket[8..12].copy_from_slice(&data.enlem.to_le_bytes());
                bucket[12..16].copy_from_slice(&data.yukseklik_mm.to_le_bytes());
                bucket[16..20].copy_from_slice(&data.hiz.to_le_bytes());
                bucket[20..24].copy_from_slice(&data.yonelim.to_le_bytes());
                let zaman_ms: u64 = embassy_time::Instant::now().as_millis() as u64;
                bucket[24..32].copy_from_slice(&zaman_ms.to_le_bytes());
                let mut calc_checksum = 0u8;
                for i in 2..32 {
                    calc_checksum ^= bucket[i];
                }
                bucket[32] = calc_checksum;
                if let Err(_) = usb_tx.write_packet(&bucket).await
                {
                    log::error!("USB Serial hatasi");
                }
            }
            Err(e) =>
            {
                log::error!("Okuma hatasi {:?}", e);
            }
        }
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner)
{
    let p = embassy_rp::init(Default::default());
    //UART BABA ALANI
    let (tx_pin, rx_pin, uart) = (p.PIN_0, p.PIN_1, p.UART0);
    static TX_BUF: StaticCell<[u8; 100]> = StaticCell::new();
    let tx_buf = &mut TX_BUF.init([0; 100])[..];
    static RX_BUF: StaticCell<[u8; 100]> = StaticCell::new();
    let rx_buf = &mut RX_BUF.init([0; 100])[..];
    let uart = BufferedUart::new(uart, tx_pin, rx_pin, Irqs, tx_buf, rx_buf, UConfig::default());    
    let (tx, rx) = uart.split();
    //USB BABA ALANI
    let usb_driver = Driver::new(p.USB, Irqs);
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
    //TASK SPAWN ALANI
    spawner.spawn(usb_task(usb_device).unwrap());
    spawner.spawn(gps_task(tx, rx, usb_baba).unwrap());
}