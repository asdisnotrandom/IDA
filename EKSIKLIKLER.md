# Görülen Hatalar ve Eksiklikler

Sistem genelinde karşılaşılan ve düzeltilmesi/güncellenmesi gereken teknik eksiklikler aşağıda listelenmiştir.

## 1. Derleme (Build) Hataları
Gömülü sistemler (ARM Cortex-M) hedeflenerek yazılan üç projede ciddi derleme hataları vardır.

* **`bno085` Projesi:**
  * **Hata:** `unresolved import core::derive`
  * **Açıklama:** `src/lib.rs` dosyasında yanlış bir import yapılmış gibi gözüküyor. `core::derive` rust içerisinde bu şekilde import edilmez, makrolar (`#[derive(...)]`) direkt olarak kullanılır.
  * **Hata (Target Mimari):** `cortex-m` kütüphanesinin (0.7.7) build sürecinde `msplim` isimli register bulunamıyor. `thumbv8m.main-none-eabihf` ve `embassy-rp` versiyon (0.10.0) ilişkilerinde bir konfigürasyon hatası var (feature flag eksikliği olabilir).

* **`gps_driver` Projesi:**
  * **Hata:** Yine `embassy-rp` kütüphanesi içerisinde `msplim` register hatası (`cortex-m` uyumsuzluğu) alınmaktadır.

* **`ida_motor_stm` Projesi:**
  * **Hata:** `no method named 'enable' found for struct 'SimplePwm'`
  * **Açıklama:** `src/main.rs` içerisinde `pwm.enable(...)` çağrısı yapılmaya çalışılmış. Ancak `SimplePwm` için bu metod `embedded_hal::Pwm` traiti (özelliği) kapsamında gelir. `use embedded_hal::Pwm;` importu dosyaya eklenmediği için derleyici bu metodu bulamamaktadır.
  * **Hata (Target Mimari):** Cortex-m assembly çağrılarında (`__basepri_r` vb.) hata dönmektedir. Bu durum hedef derleyicinin (`rustc`) veya `cortex-m` (0.7.7) ile mevcut toolchain'in tam uyumlu çalışmadığını gösteriyor. (Projede `.cargo/config.toml` içerisinde `thumbv7m-none-eabi` hedefi verilmiş ancak bağımlılıklar veya flagler uyumsuz olabilir).

## 2. Mantıksal ve Sözdizimsel Uyarılar (Clippy Warnings)
Özellikle `pi4_baba` projesi olmak üzere genel olarak aşağıdaki yapısal eksiklikler/uyarılar bulunmaktadır:

* **Kullanılmayan Değişkenler ve Fonksiyonlar (Dead Code & Unused Variables):**
  * `pi4_baba/src/beyin.rs` içerisinde tanımlanmış `nav_task` fonksiyonu ve `imu_rx`, `gps_rx`, `lidar_tx`, `motor_tx` değişkenleri oluşturulmasına rağmen kullanılmamıştır.
  * `pi4_baba/src/motorlar.rs` dosyasındaki `MotorKontrol` yapısı ve ona bağlı fonksiyonlar (`new_port`, `calc_crc8`, `set_speeds`) hiçbir yerde çağrılmamıştır.
  * `pi4_baba/src/veri_tipleri.rs` dosyasında tanımlanan veri tiplerinin (`GpsVeri`, `LidarNokta`) alanları hiçbir yerde okunmamaktadır.
  * `pi4_baba/src/main.rs` dosyasındaki `watch` importu gibi gereksiz kütüphane importları kod kalabalığı yaratmaktadır.

* **Optimizasyona Açık Bloklar (Redundant Patterns & Collapsible Ifs):**
  * `if let Ok(_) = usb_port.read_exact(...)` kullanımı yerine direkt olarak `if usb_port.read_exact(...).await.is_ok()` şeklinde daha temiz yazımlar tercih edilmelidir.
  * İç içe geçmiş `if` blokları (`if buf[0] == 0xAA` vb.), `&&` operatörü ile birleştirilerek (collapsible if) daha okunaklı ve yalın hale getirilebilir (özellikle m8n.rs ve bno085.rs içerisinde).

## 3. Genel Eksiklikler
* **`.cargo/config.toml` ve `rust-toolchain.toml`:** Depo genelinde bir `rust-toolchain.toml` (örneğin `nightly` kullanımını zorlamak için) bulunmamaktadır. Rust-embedded geliştirme ortamlarında target architecture ve spesifik rust versiyonları toolchain dosyasında belirtildiğinde projeler farklı geliştiricilerde de sorunsuz derlenebilir.
* **Hata Ayıklama (Debugging) Eksikliği:** Bazı seri haberleşme noktalarında `unwrap()` doğrudan kullanılmış. Programı anında patlatmak yerine, bağlantının koptuğunu algılayıp sistemi güvenli moda çeken (failsafe) mimari bir eksiklik vardır.
* **AGENTS.md / README.md:** Projeyi veya geliştirme ortamını anlatan herhangi bir `README.md` veya `AGENTS.md` kılavuz belgesi bulunmamaktadır. Bu durum projeyi devralan biri veya yapay zeka ajanları için dezavantajdır.