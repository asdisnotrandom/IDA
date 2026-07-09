# UBLOX M8N GPS Modulu
## Pin tanimlamalri
TX => Pin 1
RX => Pin 2

## Kullanim
Pico gps sensorunun rx tx pinlerini bagladiktan sonra, cargo run ile usb uzerinden aktarim yapilir. Picotool kullanilmasi gerekir.

## Ozellikler
115200 baudrate
10mhz okuma
sync kontrolleri
Veri tipi:
Fix_tipi u8 => 1 byte
baglanilan uydu sayisi u8 => 1 byte
enlem 4 X u8 => 4 byte
boylam 4 X u8 => 4byte
yukseklik 4x u8 => 4byte
hiz 4 X u8 => 4byte
yonelim 4xu8 => 4byte

4 bytelik bu verileri from_le_bytes ile 0 islemci maliyetiyle parse edebilirsiniz.

TIMESTAMP EKLENMELI
HEADER CRC
