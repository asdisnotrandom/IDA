import argparse
import asyncio
import json
import os
import threading
from contextlib import asynccontextmanager

import uvicorn
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

import config as cfg
from serial_manager import SerialManager, MODE_NAMES, NavData, MotData

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
static_dir = os.path.join(BASE_DIR, "static")

serial_mgr: SerialManager = None
connected_clients: set = set()
client_lock = threading.Lock()
_main_loop: asyncio.AbstractEventLoop = None


def _broadcast_sync(msg: dict):
    global connected_clients
    text = json.dumps(msg, ensure_ascii=False)
    with client_lock:
        dead = set()
        for ws in connected_clients:
            try:
                _main_loop.call_soon_threadsafe(
                    lambda ws=ws: asyncio.ensure_future(ws.send_text(text))
                )
            except:
                dead.add(ws)
        connected_clients -= dead


def packet_callback(pkt_type: str, data):
    if isinstance(data, NavData):
        msg = {
            "type": "nav",
            "lat": data.lat,
            "lon": data.lon,
            "speed": data.speed,
            "target_speed": data.target_speed,
            "roll": data.roll,
            "pitch": data.pitch,
            "yaw": data.yaw,
            "target_yaw": data.target_yaw,
            "mode": data.mode,
            "mode_name": MODE_NAMES.get(data.mode, f"Bilinmeyen ({data.mode})"),
        }
    elif isinstance(data, MotData):
        msg = {
            "type": "mot",
            "io": data.io, "ia": data.ia, "so": data.so, "sa": data.sa,
            "target_io": data.target_io, "target_ia": data.target_ia,
            "target_so": data.target_so, "target_sa": data.target_sa,
        }
    elif pkt_type == "status":
        msg = {"type": "status", **data}
    elif pkt_type == "log":
        msg = {"type": "log", **data}
    else:
        return
    _broadcast_sync(msg)


@asynccontextmanager
async def lifespan(app: FastAPI):
    global serial_mgr, _main_loop
    _main_loop = asyncio.get_running_loop()
    serial_mgr = SerialManager(cfg.SERIAL_PORT, cfg.SERIAL_BAUD)
    serial_mgr.on_packet(packet_callback)
    serial_mgr.start()

    async def status_broadcaster():
        while True:
            await asyncio.sleep(0.5)
            if serial_mgr:
                with client_lock:
                    if connected_clients:
                        status = serial_mgr.get_status()
                        msg = json.dumps({"type": "status", **status}, ensure_ascii=False)
                        await asyncio.gather(
                            *[ws.send_text(msg) for ws in connected_clients.copy()],
                            return_exceptions=True
                        )

    task = asyncio.create_task(status_broadcaster())
    yield
    task.cancel()
    if serial_mgr:
        serial_mgr.stop()


app = FastAPI(title="YKI - Yer Kontrol İstasyonu", lifespan=lifespan)

if os.path.isdir(static_dir):
    app.mount("/static", StaticFiles(directory=static_dir), name="static")


@app.get("/")
async def root():
    return FileResponse(os.path.join(static_dir, "index.html"))


@app.websocket("/ws")
async def websocket_endpoint(ws: WebSocket):
    await ws.accept()
    with client_lock:
        connected_clients.add(ws)

    if serial_mgr:
        if serial_mgr.latest_nav:
            nav = serial_mgr.latest_nav
            await ws.send_text(json.dumps({
                "type": "nav",
                "lat": nav.lat, "lon": nav.lon,
                "speed": nav.speed, "target_speed": nav.target_speed,
                "roll": nav.roll, "pitch": nav.pitch,
                "yaw": nav.yaw, "target_yaw": nav.target_yaw,
                "mode": nav.mode, "mode_name": MODE_NAMES.get(nav.mode, f"Bilinmeyen ({nav.mode})"),
            }, ensure_ascii=False))

        if serial_mgr.latest_mot:
            mot = serial_mgr.latest_mot
            await ws.send_text(json.dumps({
                "type": "mot",
                "io": mot.io, "ia": mot.ia, "so": mot.so, "sa": mot.sa,
                "target_io": mot.target_io, "target_ia": mot.target_ia,
                "target_so": mot.target_so, "target_sa": mot.target_sa,
            }, ensure_ascii=False))

        status = serial_mgr.get_status()
        await ws.send_text(json.dumps({"type": "status", **status}, ensure_ascii=False))

    try:
        while True:
            raw = await ws.receive_text()
            data = json.loads(raw)
            cmd = data.get("cmd", "")
            if cmd == "START":
                serial_mgr.send("START")
            elif cmd == "STOP":
                serial_mgr.send("STOP")
            elif cmd == "MOD":
                serial_mgr.send("MOD", data["value"])
            elif cmd == "MAN":
                serial_mgr.send("MAN", data["throttle"], data["steering"])
            elif cmd == "ROTA":
                serial_mgr.send("ROTA", data["waypoints"])
    except WebSocketDisconnect:
        pass
    except Exception:
        pass
    finally:
        with client_lock:
            connected_clients.discard(ws)


def main():
    parser = argparse.ArgumentParser(description="YKI - Yer Kontrol İstasyonu")
    parser.add_argument("--port", default=None, help=f"Seri port (varsay\u0131lan: {cfg.SERIAL_PORT})")
    parser.add_argument("--baud", type=int, default=None, help=f"Baud rate (varsay\u0131lan: {cfg.SERIAL_BAUD})")
    parser.add_argument("--host", default="0.0.0.0", help="Sunucu adresi (varsay\u0131lan: 0.0.0.0)")
    parser.add_argument("--http-port", type=int, default=8000, help="HTTP port (varsay\u0131lan: 8000)")
    args = parser.parse_args()

    port = args.port or cfg.SERIAL_PORT
    baud = args.baud or cfg.SERIAL_BAUD
    if args.port:
        os.environ["SERIAL_PORT"] = args.port
    if args.baud:
        os.environ["SERIAL_BAUD"] = str(args.baud)

    cfg.SERIAL_PORT = port
    cfg.SERIAL_BAUD = baud

    print(f"\U0001f4e1 YKI baslatiliyor...")
    print(f"   Seri port: {port} @ {baud} baud")
    print(f"   Web arayuz: http://{args.host}:{args.http_port}")
    print()

    uvicorn.run(app, host=args.host, port=args.http_port, log_level="info")


if __name__ == "__main__":
    main()
