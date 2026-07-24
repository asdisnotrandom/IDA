// ========== State ==========
const state = {
    nav: null,
    mot: null,
    connected: false,
    ws: null,
    reconnectTimer: null,
    waypoints: [],
    mapInitialized: false,
    gamepad: {
        connected: false,
        index: null,
        lastThrottle: -1,
        lastSteering: -1000,
    },
};

const MODE_NAMES = { 0: "Manuel", 1: "Otonom", 2: "G\u00f6rev Bekliyor", 3: "Acil Durum" };
const MODE_COLORS = { 0: "#58a6ff", 1: "#3fb950", 2: "#d29922", 3: "#f85149" };

// ========== Map ==========
let map, vehicleMarker, headingLine, routeLine, waypointMarkers = [];

function initMap() {
    map = L.map("map", {
        center: [41.0256, 28.9741],
        zoom: 16,
        zoomControl: true,
        attributionControl: true,
    });

    L.tileLayer("https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png", {
        maxZoom: 19,
        attribution: "&copy; OpenStreetMap",
    }).addTo(map);

    const vehicleIcon = L.divIcon({
        html: `<div class="vehicle-icon" style="width:20px;height:20px;transform:rotate(0deg)">
            <svg viewBox="0 0 20 20" width="20" height="20">
                <polygon points="10,0 3,18 10,14 17,18" fill="#58a6ff" stroke="#fff" stroke-width="0.5"/>
            </svg>
        </div>`,
        iconSize: [20, 20],
        iconAnchor: [10, 10],
        className: "",
    });

    vehicleMarker = L.marker([41.0256, 28.9741], { icon: vehicleIcon }).addTo(map);

    headingLine = L.polyline([], {
        color: "#58a6ff",
        weight: 2,
        opacity: 0.6,
        dashArray: "5,5",
    }).addTo(map);

    routeLine = L.polyline([], {
        color: "#d29922",
        weight: 3,
        opacity: 0.8,
    }).addTo(map);

    map.on("click", function (e) {
        addWaypoint(e.latlng.lat, e.latlng.lng);
    });
}

function updateVehicleMarker(lat, lon, yaw) {
    if (!vehicleMarker || !map) return;
    vehicleMarker.setLatLng([lat, lon]);

    const iconEl = vehicleMarker.getElement();
    if (iconEl) {
        const svgContainer = iconEl.querySelector(".vehicle-icon");
        if (svgContainer) {
            svgContainer.style.transform = `rotate(${yaw}deg)`;
        }
    }

    const angleRad = (yaw - 90) * Math.PI / 180;
    const length = 0.0003;
    const endLat = lat + length * Math.cos(angleRad);
    const endLon = lon + length * Math.sin(angleRad);
    headingLine.setLatLngs([
        [lat, lon],
        [endLat, endLon],
    ]);

    if (!state.mapInitialized) {
        map.setView([lat, lon], 16);
        state.mapInitialized = true;
    }
}

function updateRouteLine() {
    if (state.waypoints.length === 0) {
        routeLine.setLatLngs([]);
        return;
    }
    routeLine.setLatLngs(state.waypoints);
}

function addWaypoint(lat, lon) {
    state.waypoints.push([lat, lon]);
    renderWaypointMarkers();
    updateWaypointList();
}

function removeWaypoint(index) {
    state.waypoints.splice(index, 1);
    renderWaypointMarkers();
    updateWaypointList();
}

function renderWaypointMarkers() {
    waypointMarkers.forEach((m) => map.removeLayer(m));
    waypointMarkers = [];

    state.waypoints.forEach((wp, i) => {
        const marker = L.marker(wp, {
            icon: L.divIcon({
                html: `<div style="background:#d29922;color:#000;width:18px;height:18px;border-radius:50%;text-align:center;line-height:18px;font-size:11px;font-weight:bold;border:2px solid #fff;">${i + 1}</div>`,
                iconSize: [18, 18],
                iconAnchor: [9, 9],
                className: "",
            }),
        }).addTo(map);

        marker.on("dblclick", () => removeWaypoint(i));
        waypointMarkers.push(marker);
    });

    updateRouteLine();
}

function updateWaypointList() {
    const list = document.getElementById("waypointList");
    if (state.waypoints.length === 0) {
        list.innerHTML = '<span style="color:#8b949e">Haritaya t\u0131klayarak rota ekleyin</span>';
        return;
    }
    list.innerHTML = state.waypoints
        .map((wp, i) => `<div>${i + 1}. ${wp[0].toFixed(6)}, ${wp[1].toFixed(6)}</div>`)
        .join("");
}

function clearWaypoints() {
    state.waypoints = [];
    renderWaypointMarkers();
    updateWaypointList();
}

function sendWaypoints() {
    if (state.waypoints.length === 0) return;
    sendWS({ cmd: "ROTA", waypoints: state.waypoints });
    addLog("Rota g\u00f6nderildi: " + state.waypoints.length + " nokta", "tx");
}

// ========== WebSocket ==========
function connectWS() {
    if (state.ws && (state.ws.readyState === WebSocket.OPEN || state.ws.readyState === WebSocket.CONNECTING)) {
        return;
    }

    const protocol = window.location.protocol === "https:" ? "wss:" : "ws:";
    const wsUrl = `${protocol}//${window.location.host}/ws`;

    state.ws = new WebSocket(wsUrl);

    state.ws.onopen = function () {
        state.connected = true;
        updateStatusUI(true);
        addLog("WebSocket ba\u011flant\u0131s\u0131 kuruldu", "info");
        if (state.reconnectTimer) {
            clearTimeout(state.reconnectTimer);
            state.reconnectTimer = null;
        }
    };

    state.ws.onmessage = function (event) {
        try {
            const data = JSON.parse(event.data);
            handleMessage(data);
        } catch (e) {
            console.error("WS parse error:", e);
        }
    };

    state.ws.onclose = function () {
        state.connected = false;
        updateStatusUI(false);
        addLog("WebSocket ba\u011flant\u0131s\u0131 koptu, yeniden ba\u011flan\u0131l\u0131yor...", "error");
        state.reconnectTimer = setTimeout(connectWS, 2000);
    };

    state.ws.onerror = function () {
        state.ws.close();
    };
}

function sendWS(data) {
    if (state.ws && state.ws.readyState === WebSocket.OPEN) {
        state.ws.send(JSON.stringify(data));
    }
}

function handleMessage(data) {
    switch (data.type) {
        case "nav":
            state.nav = data;
            updateNavUI(data);
            updateVehicleMarker(data.lat, data.lon, data.yaw);
            break;
        case "mot":
            state.mot = data;
            updateMotorUI(data);
            break;
        case "status":
            updateStatusUI(data.connected);
            if (data.last_packet_ms !== undefined) {
                const info = document.getElementById("packetInfo");
                info.textContent = data.last_packet_ms >= 0
                    ? `Son paket: ${data.last_packet_ms} ms`
                    : "Son paket: ---";
            }
            break;
        case "log":
            addLog(data.message || data.message === "" ? data.message : JSON.stringify(data), "info");
            break;
    }
}

// ========== UI Updates ==========
function updateNavUI(nav) {
    setText("navLat", nav.lat.toFixed(6) + "\u00b0");
    setText("navLon", nav.lon.toFixed(6) + "\u00b0");
    setText("navSpeed", nav.speed.toFixed(2) + " m/s");
    setText("navTargetSpeed", nav.target_speed.toFixed(2) + " m/s");
    setText("navRoll", nav.roll.toFixed(1) + "\u00b0");
    setText("navPitch", nav.pitch.toFixed(1) + "\u00b0");
    setText("navYaw", nav.yaw.toFixed(1) + "\u00b0");
    setText("navTargetYaw", nav.target_yaw.toFixed(1) + "\u00b0");

    const modeEl = document.getElementById("navMode");
    const modeName = nav.mode_name || MODE_NAMES[nav.mode] || "Bilinmeyen";
    const modeColor = MODE_COLORS[nav.mode] || "#fff";
    modeEl.innerHTML = `<span style="color:${modeColor};font-weight:700">\u25cf ${modeName}</span>`;

    document.querySelectorAll(".mode-btn").forEach((btn) => {
        btn.classList.toggle("active", parseInt(btn.dataset.mode) === nav.mode);
    });
}

function updateMotorUI(mot) {
    const motors = [
        { id: "IO", actual: mot.io, target: mot.target_io },
        { id: "IA", actual: mot.ia, target: mot.target_ia },
        { id: "SO", actual: mot.so, target: mot.target_so },
        { id: "SA", actual: mot.sa, target: mot.target_sa },
    ];

    motors.forEach((m) => {
        const actualPct = Math.min(100, (m.actual / 1000) * 100);
        const targetPct = Math.min(100, (m.target / 1000) * 100);

        const barEl = document.getElementById(`motorBar${m.id}`);
        const targetBarEl = document.getElementById(`motorBarTarget${m.id}`);
        const valEl = document.getElementById(`motorVal${m.id}`);

        if (barEl) barEl.style.width = actualPct + "%";
        if (targetBarEl) targetBarEl.style.width = targetPct + "%";
        if (valEl) valEl.textContent = `${m.actual}/${m.target}`;
    });
}

function updateStatusUI(connected) {
    const dot = document.querySelector(".status-dot");
    const text = document.getElementById("statusText");

    if (connected) {
        dot.className = "status-dot connected";
        text.textContent = "Ba\u011fl\u0131";
    } else {
        dot.className = "status-dot disconnected";
        text.textContent = "Ba\u011fl\u0131 De\u011fil";
    }
}

// ========== Commands ==========
function setMode(mode) {
    sendWS({ cmd: "MOD", value: mode });
    const name = MODE_NAMES[mode] || mode;
    addLog(`Mod de\u011fi\u015ftir: ${name}`, "tx");
}

function sendCommand(cmd) {
    sendWS({ cmd: cmd });
    addLog(`Komut g\u00f6nderildi: ${cmd}`, "tx");
}

function onManualChange() {
    const throttle = parseFloat(document.getElementById("throttleSlider").value);
    const steering = parseFloat(document.getElementById("steeringSlider").value);
    document.getElementById("throttleValue").textContent = throttle.toFixed(2);
    document.getElementById("steeringValue").textContent = steering.toFixed(1) + "\u00b0";
}

function sendManual() {
    const throttle = parseFloat(document.getElementById("throttleSlider").value);
    const steering = parseFloat(document.getElementById("steeringSlider").value);
    sendWS({ cmd: "MAN", throttle: throttle, steering: steering });
    addLog(`Manuel kontrol: gaz=${throttle.toFixed(2)}, a\u00e7\u0131=${steering.toFixed(1)}\u00b0`, "tx");
}

// ========== Gamepad (Oyun Kolu) ==========
const GAMEPAD_DEADZONE = 0.15;

function updateGamepadUI(connected) {
    const dot = document.getElementById("gamepadDot");
    const text = document.getElementById("gamepadStatus");
    if (dot && text) {
        dot.className = "gamepad-dot " + (connected ? "connected" : "disconnected");
        text.textContent = connected ? "Kumanda: Bağlı" : "Kumanda: Bağlı Değil";
    }
}

function handleGamepadConnected(e) {
    state.gamepad.index = e.gamepad.index;
    state.gamepad.connected = true;
    updateGamepadUI(true);
    addLog("Oyun kolu bağlandı: " + e.gamepad.id, "info");
}

function handleGamepadDisconnected(e) {
    state.gamepad.connected = false;
    state.gamepad.index = null;
    state.gamepad.lastThrottle = -1;
    state.gamepad.lastSteering = -1000;
    updateGamepadUI(false);
    addLog("Oyun kolu bağlantısı koptu", "error");
}

function gamepadLoop() {
    const gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
    const gp = state.gamepad.index !== null ? gamepads[state.gamepad.index] : null;

    if (gp && gp.connected) {
        const axes = gp.axes;

        if (axes.length >= 2) {
            let rawLeftY = axes[1];
            let rawRightX = axes.length >= 4 ? axes[2] : axes[0];

            let throttle = 0;
            if (rawLeftY < -GAMEPAD_DEADZONE) {
                throttle = Math.min(1, (-rawLeftY - GAMEPAD_DEADZONE) / (1 - GAMEPAD_DEADZONE));
            }

            let steering = 0;
            if (Math.abs(rawRightX) > GAMEPAD_DEADZONE) {
                const sign = rawRightX > 0 ? 1 : -1;
                steering = sign * ((Math.abs(rawRightX) - GAMEPAD_DEADZONE) / (1 - GAMEPAD_DEADZONE)) * 90;
            }

            document.getElementById("throttleSlider").value = throttle;
            document.getElementById("throttleValue").textContent = throttle.toFixed(2);
            document.getElementById("steeringSlider").value = steering;
            document.getElementById("steeringValue").textContent = steering.toFixed(1) + "\u00b0";

            const mode = state.nav ? state.nav.mode : null;
            if (mode === 0) {
                if (Math.abs(throttle - state.gamepad.lastThrottle) > 0.01 ||
                    Math.abs(steering - state.gamepad.lastSteering) > 0.5) {
                    sendWS({ cmd: "MAN", throttle: throttle, steering: steering });
                    state.gamepad.lastThrottle = throttle;
                    state.gamepad.lastSteering = steering;
                }
            }
        }
    }
    requestAnimationFrame(gamepadLoop);
}

// ========== Console ==========
function addLog(message, type) {
    const log = document.getElementById("consoleLog");
    if (!log) return;

    const now = new Date();
    const time = now.toLocaleTimeString("tr-TR", { hour12: false });
    const entry = document.createElement("div");
    entry.className = "log-entry";

    const timeSpan = document.createElement("span");
    timeSpan.className = "log-time";
    timeSpan.textContent = `[${time}] `;
    entry.appendChild(timeSpan);

    const msgSpan = document.createElement("span");
    if (type === "tx") msgSpan.className = "log-tx";
    else if (type === "info") msgSpan.className = "log-info";
    else if (type === "error") msgSpan.className = "log-error";
    else if (message.startsWith("NAV:")) msgSpan.className = "log-nav";
    else if (message.startsWith("MOT:")) msgSpan.className = "log-mot";
    msgSpan.textContent = message;
    entry.appendChild(msgSpan);

    log.appendChild(entry);
    log.scrollTop = log.scrollHeight;

    // Keep last 200 entries
    while (log.children.length > 200) {
        log.removeChild(log.firstChild);
    }
}

// ========== Helpers ==========
function setText(id, text) {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
}

function toggleConsole() {
    const log = document.getElementById("consoleLog");
    const toggle = document.getElementById("consoleToggle");
    const footer = document.querySelector("footer");
    if (!log || !toggle || !footer) return;

    const isHidden = log.style.display === "none";
    log.style.display = isHidden ? "block" : "none";
    toggle.textContent = isHidden ? "\u25bc" : "\u25b2";
    footer.style.height = isHidden ? "180px" : "24px";
    const main = document.querySelector("main");
    if (main) {
        main.style.height = isHidden
            ? "calc(100vh - 48px - 180px)"
            : "calc(100vh - 48px - 24px)";
    }
}

// ========== Init ==========
document.addEventListener("DOMContentLoaded", function () {
    initMap();
    connectWS();
    onManualChange();
    updateWaypointList();
    if (navigator.getGamepads) {
        window.addEventListener("gamepadconnected", handleGamepadConnected);
        window.addEventListener("gamepaddisconnected", handleGamepadDisconnected);
        gamepadLoop();
    }
});
