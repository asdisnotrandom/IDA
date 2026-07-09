# BNO085 - Raspberry Pi Pico 2
## Kullanım
picotool gerekli
picocom, minicom, putty vb.
BOOTSEL modundaki pico 2 kartına "cargo run" ile yüklenir.

## Gönderilecek Paket Bilgisi
Toplam 47byte
Header 2
Quaternion için (Roll - 4 f32 Pitch - 4 f32 Yaw 4 f32)
Gyro için yine 4 4 4 f32
Accel için yine 4 4 4 f32
Timestamp - u64 8 byte
checksum 1

## Hata kontrol - iyilleştirme
baud rate 115200, 921600 e çekilip test edilebilir, hızı 8 kat arttırır.
timehz değişkeni mikrosaniye cinsinden alıyor, fonksiyonu yazarken hz çevirmeyi düşünerek koydum

## PINLAYOUT - test
-SCL - GPIO5 - 7. pin
-SDA - GPIO4 - 6. pin
-HINT or INT - GPIO16 - 21.pin

## KRITIK
USB Serialla yani direkt karta microusb girisinden sarj aleti takip pi4 e baglanacak. Bu kodun picoya atilmasi lazim ardindan pi4 e baglanmasi lazim. 
