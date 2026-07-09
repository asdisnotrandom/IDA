import serial
import time
import threading
import logging

# log ayarlari
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(message)s',
    datefmt='%Y-%m-%d %H:%M:%S'
)

CONFIG = {
    "rf_port": "/dev/ttyUSB0",
    "rf_baud": 57600,
    "reconnect_delay": 3.0
}

class RFTerminal:
    def __init__(self):
        # asenkron islemlerde veri bozulmasini onlemek icin kilit mekanizmasi
        self._lock = threading.Lock()
        self.keep_running = True
        self.ser_rf = None
        
        # aracin rf uzerinden gonderecegi guncel durum verileri
        self.state = {
            "mission_active": False,
            "x": 0.0,
            "y": 0.0,
            "speed": 0.0,
            "aci": 0.0,
            "battery": 100.0,
            "signal_strength": 99
        }
    
    def _calc_cs(self, payload: str) -> str:
        # basit 8-bit checksum hesaplamasi
        return f"{sum(ord(c) for c in payload) % 256:02X}"

    def connect_rf(self):
        # donanim kopmalarina karsi sonsuz baglanti deneme dongusu
        while self.keep_running:
            try:
                # timeout 0 verilerek non-blocking okuma saglanir
                self.ser_rf = serial.Serial(CONFIG["rf_port"], CONFIG["rf_baud"], timeout=0)
                logging.info(f"rf modulu ile baglanti kuruldu: {CONFIG['rf_port']}")
                return True
            except serial.SerialException:
                logging.error(f"rf portu bulunamadi. {CONFIG['reconnect_delay']} sn sonra tekrar denenecek.")
                time.sleep(CONFIG["reconnect_delay"])
        return False

    def thread_tx(self):
        # yer istasyonuna surekli veri basan iletim hatti
        logging.info("rf gonderim (tx) thread'i aktif.")
        last_fast, last_slow = 0, 0

        while self.keep_running:
            if not self.ser_rf or not self.ser_rf.is_open:
                time.sleep(0.5)
                continue

            t_now = time.time()
            
            # bellek kopyasini kilit altinda alip hizlica isleme devam et
            with self._lock:
                s = self.state.copy()

            try:
                # 10hz hizli telemetri (konum, hiz)
                if (t_now - last_fast) >= 0.1:
                    p = f"NAV:{s['x']:.2f},{s['y']:.2f},{s['speed']:.1f}"
                    self.ser_rf.write(f"{p}*{self._calc_cs(p)}\n".encode('ascii'))
                    last_fast = t_now

                # 1hz yavas telemetri (batarya, gorev durumu)
                if (t_now - last_slow) >= 1.0:
                    st = 1 if s['mission_active'] else 0
                    p = f"SYS:{s['battery']:.1f},{s['signal_strength']},{st}"
                    self.ser_rf.write(f"{p}*{self._calc_cs(p)}\n".encode('ascii'))
                    last_slow = t_now
                    
            except serial.SerialException:
                # kablo koparsa ya da modul arizalanirsa portu kapat ki rx thread'i yeniden baglanmayi tetiklesin
                logging.warning("rf yazma hatasi. baglanti dusmus olabilir.")
                self.ser_rf.close() 
                
            time.sleep(0.02) # islemcinin %100 kullanilmasini engeller

    def process_cmd(self, raw: str):
        # yer istasyonundan gelen valid komutlarin islendigi blok
        if ":" not in raw:
            return
        
        parse = raw.split(":")
        header = parse[0]
        data = parse[1]
        
        with self._lock:
            if header == "CMD":
                if data == "ARM_START":
                    self.state["mission_active"] = True
                    logging.info("gorev baslatildi - arm aktif")
                elif data == "OTO_START":
                    self.state["mission_active"] = True
                    logging.info("gorev baslatildi - otonom aktif")
                elif data == "MAN_START":
                    self.state["mission_active"] = True
                    logging.info("gorev baslatildi - kumanda aktif")
                    gaz, aci = parse[2].split(",");
                    self.state["speed"] = gaz;


                elif data == "STOP":
                    self.state["mission_active"] = False
                    logging.critical("acil durdurma - sistem pasif")  
            #elif header == "SET_POS":
                # gelen set_pos:15.5,20.0 tarzi paketleri ayirma
             #   try:
              #      x_str, y_str = data.split(",")
               #     self.state["x"] = float(x_str)
                #    self.state["y"] = float(y_str)
                 #   logging.info(f"konum manuel olarak {self.state['x']}, {self.state['y']} yapildi")
                #except ValueError:
                 #   logging.warning(f"gecersiz konum formati: {data}")

    def run(self):
        # ana islem dongusu ve alma (rx) hatti
        logging.info("rf telemetri gecidi baslatiliyor...")
        
        # tx dongusunu arka planda calistirmak uzere baslat
        tx_worker = threading.Thread(target=self.thread_tx, daemon=True)
        tx_worker.start()

        buffer = ""
        
        try:
            while self.keep_running:
                # port kapaliysa once baglanti kurulmasini bekle
                if not self.ser_rf or not self.ser_rf.is_open:
                    self.connect_rf()
                    buffer = "" # baglanti koptugunda eski yarim kalmis verileri temizle
                    continue

                # uart buffer'inda okunmayi bekleyen veri varsa al
                try:
                    if self.ser_rf.in_waiting > 0:
                        chunk = self.ser_rf.read(self.ser_rf.in_waiting).decode('ascii', errors='ignore')
                        buffer += chunk
                        
                        # veri icinde satir sonu karakteri varsa parcala
                        if "\n" in buffer:
                            lines = buffer.split("\n")
                            # son parca tamamlanmamis (orn: 'NAV:10') olabilir, bir sonraki donguye sakla
                            buffer = lines.pop() 
                            
                            for line in lines:
                                line = line.strip()
                                if not line or "*" not in line:
                                    continue
                                    
                                try:
                                    # paketi dataya ve checksum'a bol
                                    payload, cs = line.split("*", 1)
                                    if cs.upper() == self._calc_cs(payload):
                                        self.process_cmd(payload)
                                    else:
                                        logging.warning(f"veri butunlugu (checksum) hatasi: {line}")
                                except ValueError:
                                    continue
                except serial.SerialException:
                    logging.error("rf okuma hatasi. donanimin baglantisi kesildi.")
                    self.ser_rf.close()
                    
                time.sleep(0.01) # islemci duman olmasin diye kucuk bir gecikme
                
        except KeyboardInterrupt:
            logging.info("sistem kapatma komutu alindi (ctrl+c)")
        finally:
            self.keep_running = False
            if self.ser_rf and self.ser_rf.is_open:
                self.ser_rf.close()
            logging.info("portlar kapatildi. cikis basarili.")

if __name__ == "__main__":
    app = RFTerminal()
    app.run()
