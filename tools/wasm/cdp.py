"""Brauzer versiyasini ekransiz sinash: Chrome (headless) DevTools protokoli.

Ishlatish:
    python tools/wasm/cdp.py URL ssenariy.json chiqish_papkasi

Ssenariy — amallar ro'yxati:
    {"viewport": [390, 844, 1]}         # en, bo'yi, piksel zichligi
    {"open": "http://127.0.0.1:8123/app/"}
    {"wait": 6}
    {"click": [x, y]}
    {"drag": [x1, y1, x2, y2]}
    {"wheel": [x, y, dy]}
    {"type": "matn"}
    {"key": "Escape"}
    {"shot": "nom"}                      # chiqish_papkasi/nom.png
    {"eval": "JS ifoda", "print": true}

Yuklab olishlar ushlab qolinadi: `window.__dl` da fayl nomi, hajmi va
birinchi baytlari bo'ladi (haqiqiy fayl diskka yozilmaydi).
"""
import base64
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request

import websocket

CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
PORT = 9333

HOOK = """
window.__dl = [];
HTMLAnchorElement.prototype.click = function () {
  const a = this;
  const rec = { name: a.download };
  window.__dl.push(rec);
  fetch(a.href).then(r => r.arrayBuffer()).then(b => {
    rec.size = b.byteLength;
    rec.head = Array.from(new Uint8Array(b).slice(0, 4));
  });
};
"""


class Cdp:
    def __init__(self, ws_url):
        self.ws = websocket.create_connection(ws_url, timeout=120, suppress_origin=True)
        self.n = 0
        # Konsol xatolari va ushlanmagan istisnolar (panika shu yerga tushadi).
        self.errors = []

    def call(self, method, **params):
        self.n += 1
        my = self.n
        self.ws.send(json.dumps({"id": my, "method": method, "params": params}))
        while True:
            msg = json.loads(self.ws.recv())
            self.note(msg)
            if msg.get("id") == my:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result", {})

    def note(self, msg):
        m = msg.get("method")
        if m == "Runtime.exceptionThrown":
            d = msg["params"]["exceptionDetails"]
            self.errors.append("exception: " + str(d.get("exception", {}).get("description", d.get("text")))[:300])
        elif m == "Runtime.consoleAPICalled" and msg["params"].get("type") in ("error", "assert"):
            args = msg["params"].get("args", [])
            self.errors.append("console: " + " ".join(str(a.get("value", a.get("description", ""))) for a in args)[:300])

    def drain(self, seconds):
        # Kutish paytida ham hodisalarni o'qib, xatolarni yig'adi.
        end = time.time() + seconds
        self.ws.settimeout(0.5)
        while time.time() < end:
            try:
                self.note(json.loads(self.ws.recv()))
            except Exception:
                pass
        self.ws.settimeout(120)

    def mouse(self, kind, x, y, **extra):
        self.call("Input.dispatchMouseEvent", type=kind, x=x, y=y, **extra)


def main():
    scenario = json.load(open(sys.argv[1], encoding="utf-8"))
    out = sys.argv[2]
    os.makedirs(out, exist_ok=True)
    # Profil doimiy (QURAI_CDP_PROFILE): brauzer bazasi ishga tushishlar
    # orasida saqlanadi. Ko'rsatilmasa — har safar toza profil.
    profile = os.environ.get("QURAI_CDP_PROFILE") or tempfile.mkdtemp(prefix="qurai_cdp_")
    proc = subprocess.Popen(
        [
            CHROME,
            "--headless=new",
            f"--remote-debugging-port={PORT}",
            f"--user-data-dir={profile}",
            "--no-first-run",
            "--remote-allow-origins=*",
            "--enable-unsafe-swiftshader",
            "about:blank",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        for _ in range(60):
            try:
                tabs = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json"))
                page = next(t for t in tabs if t.get("type") == "page")
                break
            except Exception:
                time.sleep(0.5)
        cdp = Cdp(page["webSocketDebuggerUrl"])
        cdp.call("Page.enable")
        cdp.call("Runtime.enable")
        cdp.call("Page.addScriptToEvaluateOnNewDocument", source=HOOK)
        for step in scenario:
            if "viewport" in step:
                w, h, d = step["viewport"]
                cdp.call(
                    "Emulation.setDeviceMetricsOverride",
                    width=w, height=h, deviceScaleFactor=d, mobile=w < 768,
                )
                if w < 768:
                    cdp.call("Emulation.setTouchEmulationEnabled", enabled=True, maxTouchPoints=5)
            elif "open" in step:
                cdp.call("Page.navigate", url=step["open"])
            elif "wait" in step:
                cdp.drain(step["wait"])
            elif "click" in step:
                x, y = step["click"]
                cdp.mouse("mouseMoved", x, y)
                cdp.mouse("mousePressed", x, y, button="left", clickCount=1)
                cdp.mouse("mouseReleased", x, y, button="left", clickCount=1)
                time.sleep(0.4)
            elif "drag" in step:
                x1, y1, x2, y2 = step["drag"]
                cdp.mouse("mouseMoved", x1, y1)
                cdp.mouse("mousePressed", x1, y1, button="left", clickCount=1)
                for i in range(1, 11):
                    cdp.mouse("mouseMoved", x1 + (x2 - x1) * i / 10, y1 + (y2 - y1) * i / 10, button="left")
                    time.sleep(0.03)
                cdp.mouse("mouseReleased", x2, y2, button="left", clickCount=1)
                time.sleep(0.4)
            elif "upload" in step:
                # Fayl tanlash oynasi ushlanadi va fayl beriladi.
                x, y, path = step["upload"]
                cdp.call("Page.setInterceptFileChooserDialog", enabled=True)
                cdp.mouse("mouseMoved", x, y)
                cdp.mouse("mousePressed", x, y, button="left", clickCount=1)
                cdp.mouse("mouseReleased", x, y, button="left", clickCount=1)
                deadline = time.time() + 15
                node = None
                while time.time() < deadline and node is None:
                    msg = json.loads(cdp.ws.recv())
                    if msg.get("method") == "Page.fileChooserOpened":
                        node = msg["params"].get("backendNodeId")
                if node is None:
                    print("upload: oyna ochilmadi")
                else:
                    cdp.call("DOM.setFileInputFiles", files=[path], backendNodeId=node)
                    print("upload", path)
                cdp.call("Page.setInterceptFileChooserDialog", enabled=False)
            elif "wheel" in step:
                x, y, dy = step["wheel"]
                cdp.mouse("mouseMoved", x, y)
                cdp.mouse("mouseWheel", x, y, deltaX=0, deltaY=dy)
                time.sleep(0.5)
            elif "type" in step:
                cdp.call("Input.insertText", text=step["type"])
                time.sleep(0.3)
            elif "key" in step:
                k = step["key"]
                cdp.call("Input.dispatchKeyEvent", type="keyDown", key=k, code=k, windowsVirtualKeyCode={"Escape": 27, "Enter": 13}.get(k, 0))
                cdp.call("Input.dispatchKeyEvent", type="keyUp", key=k, code=k, windowsVirtualKeyCode={"Escape": 27, "Enter": 13}.get(k, 0))
                time.sleep(0.3)
            elif "shot" in step:
                img = cdp.call("Page.captureScreenshot", format="png")
                path = os.path.join(out, step["shot"] + ".png")
                open(path, "wb").write(base64.b64decode(img["data"]))
                print("shot", path)
            elif "eval" in step:
                r = cdp.call("Runtime.evaluate", expression=step["eval"], returnByValue=True, awaitPromise=True)
                print("eval", json.dumps(r.get("result", {}).get("value"), ensure_ascii=False))
        # Konsoldagi xatolar.
        r = cdp.call(
            "Runtime.evaluate",
            expression="JSON.stringify(window.__dl)",
            returnByValue=True,
        )
        print("downloads", r.get("result", {}).get("value"))
        for e in cdp.errors:
            print("ERROR", e)
    finally:
        proc.terminate()


if __name__ == "__main__":
    main()
