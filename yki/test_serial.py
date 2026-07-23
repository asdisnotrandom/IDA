#!/usr/bin/env python3
"""
Seri port teşhis aracı.
Kullanım: python3 test_serial.py <port>
Örnek:   python3 test_serial.py /dev/ttyACM0
"""
import sys
import serial
import time

def calc_checksum(payload: str) -> str:
    total = sum(ord(c) for c in payload)
    return f"{total % 256:02X}"

def main():
    if len(sys.argv) < 2:
        print(f"Kullanım: {sys.argv[0]} <port>")
        print(f"Örnek:    {sys.argv[0]} /dev/ttyACM0")
        sys.exit(1)

    port = sys.argv[1]
    baud = 57600

    print(f"\n🔌 {port} açılıyor... ({baud} baud)")
    try:
        ser = serial.Serial(port, baud, timeout=1.0)
        print(f"✅ Bağlantı başarılı!")
        print(f"   Port:    {ser.port}")
        print(f"   Baud:    {ser.baudrate}")
        print(f"   Durum:   {'açık' if ser.is_open else 'kapalı'}")
        print()
        print("📡 Bekleniyor... (10 saniye boyunca veri okuyacağım)")
        print("   Çıkmak için Ctrl+C")
        print()

        start = time.time()
        packet_count = 0
        while time.time() - start < 10:
            try:
                raw = ser.readline()
                if raw:
                    line = raw.decode("utf-8", errors="replace").strip()
                    if line:
                        # Checksum kontrolü
                        if "*" in line:
                            payload, cs = line.rsplit("*", 1)
                            expected = calc_checksum(payload)
                            cs_ok = "✅" if expected == cs.strip().upper() else "❌"
                            pkt_type = payload.split(":")[0] if ":" in payload else "???"
                            print(f"  {cs_ok} {pkt_type:4s} | {line}")
                        else:
                            print(f"  ⚠️  NO_CS | {line}")
                        packet_count += 1
            except Exception as e:
                print(f"  ⚠️ HATA: {e}")

        print()
        if packet_count == 0:
            print("❌ 10 saniyede hiç veri alınamadı.")
            print("   Olası sebepler:")
            print("   - Radyo modülleri eşleşmemiş olabilir")
            print("   - Araç tarafı çalışmıyor olabilir")
            print("   - Başka bir uygulama portu işgal ediyor olabilir")
        else:
            print(f"✅ {packet_count} paket alındı. Port çalışıyor!")

    except serial.SerialException as e:
        print(f"\n❌ Port açılamadı: {e}")
        print()
        print("   Olası sebepler:")
        print(f"   - '{port}' cihazı mevcut değil")
        print("   - Yetki sorunu (dialout grubunda mısın?)")
        print("   - Başka bir uygulama portu kullanıyor olabilir")
        sys.exit(1)
    finally:
        if 'ser' in locals() and ser.is_open:
            ser.close()
            print("\n🔌 Port kapatıldı.")

if __name__ == "__main__":
    main()
