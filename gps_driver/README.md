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
Header u8 => 2 byte
Fix_tipi u8 => 1 byte
baglanilan uydu sayisi u8 => 1 byte
enlem 4 X u8 => 4 byte
boylam 4 X u8 => 4byte
yukseklik 4x u8 => 4byte
hiz 4 X u8 => 4byte
yonelim 4xu8 => 4byte
timestamp 8xu8 => 8byte
crc u8 => 1 byte

TOPLAM 33 byte
