"""Dialog „KI-Server“ für Work: Adresse, API-Schlüssel und Modell einstellen.

Zweck:
    Der KI-Server (OpenAI-kompatibel, z. B. LM Studio, Ollama, llama.cpp-Server)
    und das Modell sind in der Oberfläche einstellbar statt fest im Code.
    „Verbindung testen“ fragt die Modellliste des Servers ab (GET …/models)
    und bietet die Modelle zur Auswahl an.

Genutzt von:
    katalog_gui.py
"""
import threading
import tkinter as tk
from tkinter import ttk

import bildanalyse
from bilderwerk import einstellungen   # Pfad richtet bildanalyse ein (shared/)

# Beispiele für die Adresse – nur als Hinweis im Dialog
BEISPIELE = ("LM Studio: http://127.0.0.1:1234/v1   ·   Ollama: http://127.0.0.1:11434/v1   ·   "
             "llama.cpp: http://127.0.0.1:8080/v1")


class KiDialog(tk.Toplevel):
    """Modales Fenster zur Einstellung des KI-Servers.

    Ergebnis nach dem Schließen in self.ergebnis: Dictionary
    {'url', 'modell', 'schluessel'} bei „Übernehmen“, sonst None.
    """
    def __init__(self, eltern: tk.Misc, werte: dict):
        """Dialog aufbauen.

        Parameter:
            eltern: Hauptfenster.
            werte:  Aktuelle Werte {'url', 'modell', 'schluessel'}.
        """
        super().__init__(eltern)
        self.title("KI-Server")
        self.transient(eltern)
        self.resizable(True, False)
        self.ergebnis = None
        self._test_laeuft = False

        self.var_url = tk.StringVar(value=werte.get("url", ""))
        self.var_schluessel = tk.StringVar(value=werte.get("schluessel", ""))
        self.var_modell = tk.StringVar(value=werte.get("modell", ""))

        pad = {"padx": 8, "pady": 4}
        r = ttk.Frame(self)
        r.pack(fill="both", expand=True, padx=10, pady=10)
        ttk.Label(r, text="Adresse:").grid(row=0, column=0, sticky="w", **pad)
        ttk.Entry(r, textvariable=self.var_url, width=50).grid(row=0, column=1, sticky="ew", **pad)
        ttk.Label(r, text=BEISPIELE, foreground="#777777", font=("", 10)) \
            .grid(row=1, column=1, sticky="w", padx=8)
        ttk.Label(r, text="API-Schlüssel:").grid(row=2, column=0, sticky="w", **pad)
        ttk.Entry(r, textvariable=self.var_schluessel, show="•") \
            .grid(row=2, column=1, sticky="ew", **pad)
        ttk.Label(r, text="nur nötig, wenn der Server einen verlangt (lokal meist leer)",
                  foreground="#777777", font=("", 10)).grid(row=3, column=1, sticky="w", padx=8)
        ttk.Label(r, text="Modell:").grid(row=4, column=0, sticky="w", **pad)
        self.cmb_modell = ttk.Combobox(r, textvariable=self.var_modell)
        self.cmb_modell.grid(row=4, column=1, sticky="ew", **pad)
        ttk.Label(r, text="Das Modell muss Bilder verstehen (Vision) – z. B. qwen/qwen3.5-9b.",
                  foreground="#777777", font=("", 10)).grid(row=5, column=1, sticky="w", padx=8)

        test = ttk.Frame(r)
        test.grid(row=6, column=1, sticky="w", **pad)
        self.btn_test = ttk.Button(test, text="Verbindung testen", command=self._testen)
        self.btn_test.pack(side="left")
        self.lbl_test = ttk.Label(test, text="", wraplength=420)
        self.lbl_test.pack(side="left", padx=10)
        r.columnconfigure(1, weight=1)

        knoepfe = ttk.Frame(r)
        knoepfe.grid(row=7, column=0, columnspan=2, sticky="e", pady=(10, 0))
        ttk.Button(knoepfe, text="Abbrechen", command=self.destroy).pack(side="right")
        ttk.Button(knoepfe, text="Übernehmen", command=self._uebernehmen) \
            .pack(side="right", padx=6)
        self.bind("<Escape>", lambda _e: self.destroy())
        self.grab_set()

    def _testen(self):
        """Modellliste im Hintergrund abfragen und das Ergebnis anzeigen.

        Rückgabe:
            Keine.
        """
        if self._test_laeuft:
            return
        self._test_laeuft = True
        self.btn_test.configure(state="disabled")
        self.lbl_test.configure(text="Teste …", foreground="")
        url, schluessel = self.var_url.get().strip(), self.var_schluessel.get().strip()

        def arbeit():
            try:
                ergebnis = ("ok", bildanalyse.modelle_abfragen(url, schluessel))
            except bildanalyse.ServerNichtErreichbar as ex:
                ergebnis = ("fehler", f"Server nicht erreichbar ({ex}). Läuft er?")
            except Exception as ex:
                ergebnis = ("fehler", str(ex))
            try:
                self.after(0, lambda: self._test_fertig(*ergebnis))
            except (RuntimeError, tk.TclError):
                pass                           # Dialog inzwischen geschlossen

        threading.Thread(target=arbeit, daemon=True).start()

    def _test_fertig(self, art: str, wert):
        """Ergebnis des Verbindungstests anzeigen (im Tk-Hauptthread).

        Parameter:
            art:  'ok' oder 'fehler'.
            wert: Modellliste bzw. Fehlermeldung.
        """
        self._test_laeuft = False
        if not self.winfo_exists():
            return
        self.btn_test.configure(state="normal")
        if art == "fehler":
            self.lbl_test.configure(text=f"✗ {wert}", foreground="#c00000")
            return
        self.cmb_modell.configure(values=wert)
        if not wert:
            self.lbl_test.configure(text="✓ Verbunden, aber der Server meldet keine Modelle "
                                         "(Modell geladen?)", foreground="#b35c00")
            return
        if not self.var_modell.get().strip() and len(wert) == 1:
            self.var_modell.set(wert[0])
        hinweis = "" if self.var_modell.get().strip() in wert else \
            " – eingetragenes Modell nicht dabei, bitte auswählen"
        self.lbl_test.configure(text=f"✓ Verbunden, {len(wert)} Modell(e){hinweis}",
                                foreground="#b35c00" if hinweis else "#2e7d32")

    def _uebernehmen(self):
        """Werte übernehmen und Dialog schließen.

        Rückgabe:
            Keine.
        """
        if not self.var_url.get().strip():
            self.lbl_test.configure(text="✗ Bitte eine Adresse eintragen.", foreground="#c00000")
            return
        self.ergebnis = {"url": einstellungen.ki_url_normalisieren(self.var_url.get()),
                         "modell": self.var_modell.get().strip(),
                         "schluessel": self.var_schluessel.get().strip()}
        self.destroy()
