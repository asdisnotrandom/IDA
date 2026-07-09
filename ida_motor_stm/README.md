## Pın Layout
TIM2 ile CH1 CH2 CH3 CH4
PA0 PA1 PA2 PA3
iskeleon iskelearka sancakon sancakarka
Uart baudrate 115200
CRC8 ile veri kontrolü (ilk iki byte 0xAA ve 0x55 olmalı, değişebilir)
sonunda, tıpkı koddaki gibi 1 bytelık crc hesaplaması bulunmalıdır.
PID hesaplaması, sensörlerin ortak katmanı olması dolayısıyla pi4 te halledilmeli ve bu karta artık işlenmiş ve direkt motorlara uygulanacak pwmler gönderilmeli.

## veri gönderimi
[0xAA, 0x55, iskeleon, iskelearka, sancakon, sancakarka, crc8]

### ÖNEMLİ!!!
-> motor pwm degerleri 0 ile 1000 arasında (0,1000 dahil gönderilmeli.) ESC ile bldc motorlar için en hassas ayarlamaları sağlar. yüzde 50 yük için 500 degeri girilmeli.

## kulanım
probe rs ile rahatça cargo run komuduyla karta yüklenebilir, usb izinlerine dikkat. 
