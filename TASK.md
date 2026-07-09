# İnsansız Deniz Aracı (İDA) Dikdörtgen Çizme Algoritması (Görev Planı)

Bu belge, elde bulunan `GpsVeri`, `ImuVeri`, `LidarVeri` ve `MotorVeri` kullanılarak insansız deniz aracının deniz üzerinde bir dikdörtgen rotası izlemesini sağlamak için izlenmesi gereken adımları açıklamaktadır. Kod yazılmayacak olup, algoritma `nav_task` içerisinde tasarlanacak şekilde aşağıda adımlandırılmıştır.

## 1. Hazırlık ve Sensör Verilerinin Entegrasyonu
- **Sensör Kanallarının Dinlenmesi:** `nav_task` fonksiyonunda `mpsc::Receiver` kanalları üzerinden `GpsVeri`, `ImuVeri` ve `LidarVeri` sürekli veya event-bazlı olarak dinlenmelidir.
- **Konum ve Yönelim Güncellemeleri:**
  - IMU verisinden (özellikle `yaw` açısından) aracın anlık baş (heading) yönü sürekli takip edilmelidir.
  - GPS verisinden (özellikle `enlem` ve `boylam`) aracın o anki mutlak konumu belirlenmelidir.

## 2. Navigasyon Durum Makinesi (State Machine) Kurulumu
Dikdörtgen çizimi, dört düz gidiş (kenar) ve dört dönüş (köşe) aşamasından oluştuğu için bir Durum Makinesi (State Machine) oluşturulmalıdır.
- **Durumlar (States):**
  1. `BaslangicNoktasinaGit`: (Opsiyonel) Referans noktasına ulaştıktan sonra görev başlatılır.
  2. `KenarCiz (Kenar No)`: Belirli bir mesafe veya koordinat noktasına (hedef GPS noktası) doğru düz gidilmesi. (No = 1, 2, 3, 4)
  3. `KoseDonus (Kose No)`: Aracın 90 derece dönmesi. (No = 1, 2, 3, 4)
  4. `GorevTamamlandi`: Araç motorlarını durdurup bekleme konumuna geçer.

## 3. Hedef (Waypoint) Hesaplamaları
- Başlangıç GPS koordinatı alındıktan sonra, dikdörtgenin uzunluk (ör: 50m) ve genişlik (ör: 20m) parametrelerine göre diğer 3 köşe noktasının GPS (enlem/boylam) karşılıkları bir formül (Ör: Haversine veya yerel düzlemsel yaklaşımlar) kullanılarak hesaplanıp kaydedilmelidir.

## 4. `nav_task` Ana Döngü (Control Loop) İşleyişi
Döngü içerisinde sensör verileri okundukça mevcut Durum'a (State) göre kontrol tepkileri verilir:

### A. İleri Gidiş Durumu (`KenarCiz`)
1. **Mesafe ve Yön Kontrolü:** O anki GPS pozisyonu ile hedef köşe noktası arasındaki mesafe ve gereken yön (bearing) hesaplanır.
2. **PID Kontrol (Heading Control):** Aracın anlık yönü (IMU `yaw`) ile hedef yöne gitmek için gereken yön (Bearing) arasındaki fark (hata) hesaplanır.
3. **Motor Komutları:** PID çıktısı `MotorVeri` üzerine uygulanır (İskele ve Sancak motorlarına diferansiyel itki - differential thrust - vererek rotada düz tutulur).
4. **Durum Geçişi:** Hedef noktaya belirlenen hata toleransı mesafesinde (örneğin 2 metre) yaklaşıldığında durum `KoseDonus` durumuna güncellenir.

### B. Köşe Dönüşü Durumu (`KoseDonus`)
1. **Dönüş Hedefi Belirleme:** Mevcut IMU `yaw` açısına 90 derece (veya istenilen dönüş yönüne göre -90 derece) eklenerek yeni hedef baş açısı bulunur.
2. **Yerinde Dönüş (Pivot Turn):** `MotorVeri` yapısındaki İskele (sol) ve Sancak (sağ) motorları ters yönlerde çalıştırılarak araca yerinde (veya kısa kavisle) dönüş yaptırılır.
3. **Durum Geçişi:** IMU `yaw` açısı hedef açıya geldiğinde (küçük bir tolerans payı bırakılarak) durum bir sonraki `KenarCiz` adımına güncellenir.

## 5. Güvenlik ve Engel Aşma (Lidar Entegrasyonu)
- `KenarCiz` veya `KoseDonus` sırasında `LidarVeri` sürekli kontrol edilir.
- `LidarNokta` objelerinden `aci` değeri aracın ilerleme yönüne yakın (örneğin ön +/- 30 derece) ve `mesafe_mm` belirli bir tehlike eşiğinin altındaysa (örneğin 5 metre):
  - Sistem "Acil Durdurma" veya "Engel Aşma (Obstacle Avoidance)" alt-durumuna geçmelidir.
  - Bu durumda motorlar durdurulmalı (tüm ESC'ler bekleme devrine çekilmeli) veya rotadan saparak engelin etrafından dolanacak yeni bir "waypoint" belirlenmelidir.

## 6. Motor Verilerinin Gönderilmesi
Tüm bu durum hesaplamalarının ardından nihai motor güç değerleri `MotorVeri` objesine atanır. Bu obje, `watch::Sender<MotorVeri>` üzerinden (kod yapısındaki `motor_tx`) motor kontrol task'ına iletilerek donanımsal komuta dönüşür. (Ana fonksiyonda `main`, sadece `nav_task` dahil diğer taskleri tokio threadleri olarak baslatip kapatır.)

Bu mantıksal plan çerçevesinde kod modüler olarak yazıldığı takdirde, İnsansız Deniz Aracı sadece kendisine gelen sensör akışlarını dinleyip tepki vererek otonom dikdörtgen görevini başarabilir.