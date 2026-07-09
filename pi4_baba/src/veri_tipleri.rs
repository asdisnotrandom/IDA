#[derive(Debug, Clone)]
pub struct MotorVeri
{
    pub iskeleon: u16,
    pub iskelearka: u16,
    pub sancakon: u16,
    pub sancakarka: u16,
}
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ImuVeri
{
    pub roll: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub gx: f32,
    pub gy: f32,
    pub gz: f32,
    pub ax: f32,
    pub ay: f32,
    pub az: f32,
    pub zaman_ms: u64,
}
pub struct GpsVeri
{
    pub algi_boyut: u8,
    pub uydu_sayi: u8,
    pub boylam: i32,
    pub enlem: i32,
    pub yukseklik_mm: i32,
    pub hiz: i32,
    pub yonelim: i32,
    pub zaman_ms: u64,

}
#[derive(Clone,Debug)]
pub struct LidarNokta
{
    pub aci: f32,
    pub mesafe_mm: f32,
    pub kalite: u8
}
pub struct LidarVeri
{
    pub noktalar: Vec<LidarNokta>,
    pub zaman_ms: u64,
}