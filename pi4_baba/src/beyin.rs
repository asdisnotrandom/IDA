use tokio::{sync::{mpsc, watch}, time::interval};
use crate::veri_tipleri::{GpsVeri,ImuVeri,LidarVeri, MotorVeri};
use std::{f32::consts::PI, time::{Duration, Instant}};

pub struct PidKontrolcu
{
    pub kp: f32,
    pub ki: f32,
    pub kd: f32,
    integral: f32,
    onceki_hata: f32,
    integral_siniri: f32,
}

impl PidKontrolcu {
    pub fn new(kp: f32, ki: f32, kd: f32, integral_siniri: f32) -> Self
    {
        Self
        {
            kp,
            ki,
            kd,
            integral: 0.0,
            onceki_hata: 0.0,
            integral_siniri,
        }
    }

    pub fn guncelle(&mut self, hata: f32, dt: f32) -> f32
    {
        self.integral += hata * dt;
        self.integral = self.integral.clamp(-self.integral_siniri, self.integral_siniri);
        let turev = if dt > 0.0 { (hata - self.onceki_hata) / dt } else { 0.0 };
        self.onceki_hata = hata;
        (self.kp * hata) + (self.ki * self.integral) + (self.kd * turev)
    }
}

const HEDEF1X: f32 = 0.0;
const HEDEF1Y: f32 = 0.0;
const HEDEF2X: f32 = 0.0;
const HEDEF2Y: f32 = 0.0;
const HEDEF3X: f32 = 0.0;
const HEDEF3Y: f32 = 0.0;
const HEDEF4X: f32 = 0.0;
const HEDEF4Y: f32 = 0.0;

const IHMALACI: f32 = 3.0;
const HEDEF_TOLERANS: f32 = 2.5;

pub struct NavData
{
    origin_enlem: f32,
    origin_boylam: f32,
    cos_enlem: f32,
    is_origin_set: bool,
    gps_ornek_sayaci: u8,
    ornek_enlem_toplam: i64,
    ornek_boylam_toplam: i64,
    current_x: f32,
    current_y: f32,
    current_yaw: f32,
    hedef_noktalar: Vec<(f32, f32)>,
    current_hn_index: usize,
}

impl NavData
{
    pub fn new() -> Self
    {
        Self
        {
            origin_enlem: 0.0,
            origin_boylam: 0.0,
            cos_enlem: 1.0,
            is_origin_set: false,
            gps_ornek_sayaci: 0,
            ornek_enlem_toplam: 0,
            ornek_boylam_toplam: 0,
            current_x: 0.0,
            current_y: 0.0,
            current_yaw: 0.0,
            hedef_noktalar: vec![
                (HEDEF1X, HEDEF1Y),
                (HEDEF2X, HEDEF2Y),
                (HEDEF3X, HEDEF3Y),
                (HEDEF4X, HEDEF4Y),
            ],
            current_hn_index: 0,
        }
    }
    pub fn guvenli_origin_belirle(&mut self, gps: &GpsVeri) -> bool {
        let yeterli_fix = gps.algi_boyut >= 3; 
        let yeterli_uydu = gps.uydu_sayi >= 6;
        let gecerli_koordinat = gps.enlem != 0 && gps.boylam != 0;
        if !yeterli_fix || !yeterli_uydu || !gecerli_koordinat {
            self.gps_ornek_sayaci = 0;
            self.ornek_enlem_toplam = 0;
            self.ornek_boylam_toplam = 0;
            return false;
        }
        self.ornek_enlem_toplam += gps.enlem as i64;
        self.ornek_boylam_toplam += gps.boylam as i64;
        self.gps_ornek_sayaci += 1;
        if self.gps_ornek_sayaci >= 10 {
            let ortalama_enlem = (self.ornek_enlem_toplam / 10) as i32;
            let ortalama_boylam = (self.ornek_boylam_toplam / 10) as i32;
            self.origin_enlem = ortalama_enlem as f32 / 10_000_000.0;
            self.origin_boylam = ortalama_boylam as f32 / 10_000_000.0;
            self.cos_enlem = (self.origin_enlem * std::f32::consts::PI / 180.0).cos();
            self.is_origin_set = true;
            return true;
        }
        false
    }
    pub fn guncelle_konum(&mut self, lat_i32: i32, lon_i32: i32, yaw: f32)
    {
        let lat = lat_i32 as f32 / 10_000_000.0;
        let lon = lon_i32 as f32 / 10_000_000.0;
        self.current_y = (lat - self.origin_enlem) * 111_320.0;
        self.current_x = (lon - self.origin_boylam) * 111_320.0 * self.cos_enlem;
        self.current_yaw = yaw;
    }
    pub fn guncel_hedef(&self) -> Option<(f32, f32)> {
        if self.current_hn_index < self.hedef_noktalar.len() {
            Some(self.hedef_noktalar[self.current_hn_index])
        } else {
            None
        }
    }
    pub fn calc_mesafe(&self, hedef_x: f32, hedef_y: f32) -> f32 {
        let dx = hedef_x - self.current_x;
        let dy = hedef_y - self.current_y;
        (dx * dx + dy * dy).sqrt()
    }
    pub fn calc_hedefeaci(&self, hedef_x: f32, hedef_y: f32) -> f32 {
        let dx = hedef_x - self.current_x;
        let dy = hedef_y - self.current_y;
        let mut yonelimaci = dx.atan2(dy) * 180.0 / PI;  
        if yonelimaci < -180.0 { yonelimaci += 360.0; }
        if yonelimaci > 180.0 { yonelimaci -= 360.0; }
        yonelimaci
    }
    pub fn bakisyonu_hata(&self, hedefeaci: f32) -> f32 {
        let mut hata = hedefeaci - self.current_yaw;
        while hata > 180.0 { hata -= 360.0 }
        while hata < -180.0 { hata += 360.0 }
        hata
    }
}

pub async fn nav_task (
    imu_rx: watch::Receiver<ImuVeri>,
    gps_rx: watch::Receiver<GpsVeri>,
    motor_tx: mpsc::Sender<MotorVeri>,
    )
{
    let mut tick = interval(Duration::from_millis(50));
    let mut last_time = Instant::now();
    let mut nav = NavData::new();
    let mut pid = PidKontrolcu::new(4.0, 0.1, 0.5, 150.0);
    let base_hiz = 400.0;
    let mut kaba_donus_modu = false;
    loop {
        tick.tick().await;
        let simdi = Instant::now();
        let dt = simdi.duration_since(last_time).as_secs_f32();
        last_time = simdi;
        let gps = gps_rx.borrow().clone();
        let imu = imu_rx.borrow().clone();
        if !nav.is_origin_set
        {
            let _hazir = nav.guvenli_origin_belirle(&gps);
            let _ = motor_tx.send(MotorVeri { iskeleon: 0, iskelearka: 0, sancakon: 0, sancakarka: 0 }).await;
            continue;
        }
        nav.guncelle_konum(gps.enlem, gps.boylam, imu.yaw);
        if let Some((hedef_x, hedef_y)) = nav.guncel_hedef() {
            let mesafe = nav.calc_mesafe(hedef_x, hedef_y);
            let hedefe_aci = nav.calc_hedefeaci(hedef_x, hedef_y);
            let hata = nav.bakisyonu_hata(hedefe_aci);
            if mesafe < HEDEF_TOLERANS {
                println!("{}. hedef noktasına ulaşıldı!", nav.current_hn_index + 1);
                nav.current_hn_index += 1;
                pid.integral = 0.0;
                pid.onceki_hata = hata;
                kaba_donus_modu = false;
                continue;
            }
            if hata.abs() > 30.0
            {
                kaba_donus_modu = true;
            }
            if hata.abs() < 15.0
            {
                kaba_donus_modu = false;
            }
            let donus_gucu = pid.guncelle(hata, dt);
            let mut iskele_on = 0;
            let mut sancak_on = 0;
            let mut iskele_arka = 0;
            let mut sancak_arka = 0;
            if kaba_donus_modu
            {
                if hata < 0.0
                {
                    sancak_on = 400;
                }
                else {
                    iskele_on = 400;
                }
            }
            else {
                let duzeltme = if hata.abs() < IHMALACI { 0.0 } else { donus_gucu };
                iskele_arka = (base_hiz - duzeltme).clamp(0.0, 1000.0) as u16;
                sancak_arka = (base_hiz + duzeltme).clamp(0.0, 1000.0) as u16;
            }
            let _ = motor_tx.send(MotorVeri {
                iskeleon: iskele_on,
                iskelearka: iskele_arka,
                sancakon: sancak_on,
                sancakarka: sancak_arka,
            }).await;
        } else {
            let _ = motor_tx.send(MotorVeri {
                iskeleon: 0,
                iskelearka: 0,
                sancakon: 0,
                sancakarka: 0,
            }).await;
        }
    }

}