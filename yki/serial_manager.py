import threading
import serial
import time
import re
from collections import deque
from dataclasses import dataclass, field
from typing import Optional


@dataclass
class NavData:
    lat: float = 0.0
    lon: float = 0.0
    speed: float = 0.0
    target_speed: float = 0.0
    roll: float = 0.0
    pitch: float = 0.0
    yaw: float = 0.0
    target_yaw: float = 0.0
    mode: int = 0


@dataclass
class MotData:
    io: int = 0
    ia: int = 0
    so: int = 0
    sa: int = 0
    target_io: int = 0
    target_ia: int = 0
    target_so: int = 0
    target_sa: int = 0


MODE_NAMES = {0: "Manuel", 1: "Otonom", 2: "Görev Bekliyor", 3: "Acil Durum"}


def calc_checksum(payload: str) -> str:
    total = sum(ord(c) for c in payload)
    return f"{total % 256:02X}"


def checksum_validate(line: str) -> Optional[str]:
    line = line.strip()
    if "*" not in line:
        return None
    payload, cs = line.rsplit("*", 1)
    if calc_checksum(payload) == cs.strip().upper():
        return payload
    return None


def parse_nav(payload: str) -> Optional[NavData]:
    if not payload.startswith("NAV:"):
        return None
    fields = payload[4:].split(",")
    if len(fields) < 9:
        return None
    try:
        return NavData(
            lat=float(fields[0]),
            lon=float(fields[1]),
            speed=float(fields[2]),
            target_speed=float(fields[3]),
            roll=float(fields[4]),
            pitch=float(fields[5]),
            yaw=float(fields[6]),
            target_yaw=float(fields[7]),
            mode=int(fields[8]),
        )
    except (ValueError, IndexError):
        return None


def parse_mot(payload: str) -> Optional[MotData]:
    if not payload.startswith("MOT:"):
        return None
    fields = payload[4:].split(",")
    if len(fields) < 8:
        return None
    try:
        return MotData(
            io=int(fields[0]), ia=int(fields[1]), so=int(fields[2]), sa=int(fields[3]),
            target_io=int(fields[4]), target_ia=int(fields[5]), target_so=int(fields[6]), target_sa=int(fields[7]),
        )
    except (ValueError, IndexError):
        return None


def format_cmd(payload: str) -> str:
    return f"{payload}*{calc_checksum(payload)}\n"


class SerialManager:
    def __init__(self, port: str, baud: int):
        self.port = port
        self.baud = baud
        self.ser: Optional[serial.Serial] = None
        self.running = False
        self._thread: Optional[threading.Thread] = None
        self._lock = threading.Lock()

        self.latest_nav: Optional[NavData] = None
        self.latest_mot: Optional[MotData] = None
        self.last_packet_time: float = 0.0
        self.connected = False
        self._on_packet = None

        self._write_queue: deque = deque()
        self._write_event = threading.Event()

    def on_packet(self, callback):
        self._on_packet = callback

    def _process_line(self, line: str):
        payload = checksum_validate(line)
        if payload is None:
            return

        nav = parse_nav(payload)
        if nav is not None:
            with self._lock:
                self.latest_nav = nav
                self.last_packet_time = time.time()
            if self._on_packet:
                self._on_packet("nav", nav)
            return

        mot = parse_mot(payload)
        if mot is not None:
            with self._lock:
                self.latest_mot = mot
                self.last_packet_time = time.time()
            if self._on_packet:
                self._on_packet("mot", mot)

    def _log(self, msg: str):
        import sys
        print(f"[YKI] {msg}", file=sys.stderr)

    def _run(self):
        while self.running:
            try:
                self.ser = serial.Serial(self.port, self.baud, timeout=0.05)
                self.connected = True
                self._log(f"Seri port açıldı: {self.port} @ {self.baud} baud")
                if self._on_packet:
                    self._on_packet("status", {"connected": True, "message": f"Seri porta bağlanıldı: {self.port}"})

                while self.running:
                    # Send pending commands
                    while self._write_queue:
                        cmd = self._write_queue.popleft()
                        if self.ser and self.ser.is_open:
                            self.ser.write(cmd.encode("utf-8"))
                            if self._on_packet:
                                self._on_packet("log", {"message": f"→ {cmd.strip()} gönderildi"})

                    # Read line (blocking with timeout)
                    try:
                        raw = self.ser.readline()
                    except serial.SerialException:
                        raise

                    if not raw:
                        continue

                    try:
                        line = raw.decode("utf-8", errors="replace").strip()
                    except Exception as e:
                        self._log(f"decode hatası: {e}")
                        continue

                    if not line:
                        continue

                    self._process_line(line)

            except serial.SerialException as e:
                self.connected = False
                self._log(f"Seri port hatası: {e}")
                if self._on_packet:
                    self._on_packet("status", {"connected": False, "message": f"Seri port hatası: {e}"})
                time.sleep(2)
            except Exception as e:
                self.connected = False
                self._log(f"Beklenmeyen hata: {e}")
                if self._on_packet:
                    self._on_packet("status", {"connected": False, "message": f"Hata: {e}"})
                time.sleep(2)
            finally:
                if self.ser and self.ser.is_open:
                    try:
                        self.ser.close()
                    except:
                        pass

    def start(self):
        if self.running:
            return
        self.running = True
        self._thread = threading.Thread(target=self._run, daemon=True)
        self._thread.start()

    def stop(self):
        self.running = False
        if self.ser and self.ser.is_open:
            try:
                self.ser.close()
            except:
                pass

    def send(self, cmd_type: str, *args):
        if cmd_type == "START":
            payload = "CMD:START"
        elif cmd_type == "STOP":
            payload = "CMD:STOP"
        elif cmd_type == "MOD":
            payload = f"CMD:MOD:{args[0]}"
        elif cmd_type == "MAN":
            payload = f"CMD:MAN:{args[0]},{args[1]}"
        elif cmd_type == "ROTA":
            points = ";".join(f"{lat},{lon}" for lat, lon in args[0])
            payload = f"CMD:ROTA:{points}"
        else:
            return
        self._write_queue.append(format_cmd(payload))

    def get_status(self) -> dict:
        with self._lock:
            elapsed = time.time() - self.last_packet_time if self.last_packet_time > 0 else -1
            return {
                "connected": self.connected,
                "last_packet_ms": int(elapsed * 1000) if elapsed >= 0 else -1,
                "port": self.port,
                "baud": self.baud,
            }
