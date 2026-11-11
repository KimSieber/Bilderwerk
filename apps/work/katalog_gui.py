#!/usr/bin/env python3
"""Bilderkatalog – Oberfläche für den Katalogisierungslauf (Work).

Zweck:
    Tkinter-Oberfläche um katalog.Katalogisierer: Bildordner, Datenbank und
    Sicherung wählen, Lauf starten / pausieren / stoppen, Fortschritt und
    Restzeit verfolgen, das zuletzt analysierte Bild mit Vorschau und
    Ergebnis ansehen, Meldungen und Protokoll einsehen.

Ziel:
    Bequemer, unbeaufsichtigter Dauerlauf (Ruhezustand wird per caffeinate
    verhindert). Pause/Stopp wirken jeweils nach dem aktuellen Bild; der
    Fortschritt ist jederzeit gespeichert.

Aufruf:
    python3 katalog_gui.py   (oder Doppelklick auf „Bilderwerk Work starten.command“)

Einstellungen:
    Gemeinsame Datei von Work und Sweep im Benutzerordner
    (~/Library/Application Support/Bilderwerk/einstellungen.json, siehe
    shared/bilderwerk/einstellungen.py): Datenbank, Abschnitt 'work' mit
    Bildordner, Sicherung, Optionen und KI-Server. Wird beim Start geladen und
    bei jeder Änderung bzw. beim Start eines Laufs gespeichert. Beim ersten
    Start werden die Werte der alten gui_einstellungen.json übernommen.

Threads:
    Der Lauf läuft in einem eigenen Thread; Meldungen kommen über eine Queue,
    die Anzeige wird jede Sekunde im Tk-Hauptthread aktualisiert.
"""
import os
import queue
import shutil
import subprocess
import sys
import tempfile
import threading
import time
import sqlite3
import tkinter as tk
from datetime import datetime, timedelta
from pathlib import Path
from tkinter import filedialog, messagebox, ttk
from tkinter.scrolledtext import ScrolledText

import katalog
import metadaten

import bildanalyse
from bilderwerk import einstellungen   # Pfad richtet bildanalyse ein (shared/)
from ki_dialog import KiDialog

VORSCHAU_PX = 320


def dauer_text(sekunden: float) -> str:
    """Sekunden als gut lesbare Dauer formatieren.

    Parameter:
        sekunden: Dauer in Sekunden.

    Rückgabe:
        z. B. '2 T 5 h', '3 h 12 min', '7 min'; '–' bei 0 oder weniger.
    """
    if sekunden <= 0:
        return "–"
    t, rest = divmod(int(sekunden), 86400)
    h, rest = divmod(rest, 3600)
    m = rest // 60
    if t:
        return f"{t} T {h} h"
    if h:
        return f"{h} h {m} min"
    return f"{m} min"


def zahl(n: int) -> str:
    """Ganzzahl mit deutschem Tausenderpunkt formatieren.

    Parameter:
        n: Zahl.

    Rückgabe:
        z. B. '81.000'.
    """
    return f"{n:,}".replace(",", ".")


class KatalogGUI:
    """Hauptfenster der Katalogisierung.

    Hält die Tk-Variablen der Einstellungen, den laufenden Katalogisierer samt
    Arbeits-Thread, den optionalen caffeinate-Prozess und die Vorschau.
    """
    def __init__(self, root: tk.Tk):
        """Fenster aufbauen, Einstellungen laden und Anzeigeschleife starten.

        Parameter:
            root: Tk-Hauptfenster.
        """
        self.root = root
        self.root.title("Bilderwerk Work – Katalogisierung")
        self.root.minsize(980, 720)
        self.meldungen: queue.Queue[str] = queue.Queue()
        self.katalogisierer: katalog.Katalogisierer | None = None
        self.thread: threading.Thread | None = None
        self.caffeinate: subprocess.Popen | None = None
        self.letzter_vorschau_pfad = None
        self.vorschau_bild = None   # Referenz halten, sonst raeumt Tk das Bild ab

        self._oberflaeche_bauen()
        self._einstellungen_laden()
        self._db_info_zeigen()
        self.root.protocol("WM_DELETE_WINDOW", self._schliessen)
        self._aktualisieren()

    # ------------------------------------------------------------ Aufbau
    def _oberflaeche_bauen(self):
        """Alle Bedienelemente anlegen.

        Bereiche: Einstellungen (Ordner, Datenbank, Sicherung, Optionen),
        Steuerleiste (Start/Pause/Stopp, Protokoll), Fortschritt (Balken und
        Kennzahlen), „Zuletzt analysiert“ (Vorschau + Ergebnis) und Meldungen.

        Rückgabe:
            Keine.
        """
        pad = {"padx": 6, "pady": 3}

        # --- Einstellungen ---
        rahmen = ttk.LabelFrame(self.root, text="Einstellungen")
        rahmen.pack(fill="x", padx=10, pady=(10, 5))
        self.var_ordner = tk.StringVar()
        self.var_db = tk.StringVar(value=str(bildanalyse.STANDARD_DB))
        self.var_backup = tk.StringVar()
        self.var_backup_alle = tk.IntVar(value=katalog.BACKUP_ALLE)
        self.var_wach = tk.BooleanVar(value=True)
        self.ki = {"url": bildanalyse.API_URL, "modell": bildanalyse.MODEL,
                   "schluessel": bildanalyse.API_SCHLUESSEL}

        zeilen = [("Bildordner:", self.var_ordner, [("Wählen …", self._ordner_waehlen)]),
                  ("Datenbank:", self.var_db, [("Vorhandene öffnen …", self._db_oeffnen),
                                               ("Neu anlegen …", self._db_neu)]),
                  ("Sicherung (optional):", self.var_backup,
                   [("Ziel wählen …", self._backup_waehlen)])]
        for i, (text, var, knoepfe) in enumerate(zeilen):
            ttk.Label(rahmen, text=text).grid(row=i * 2, column=0, sticky="w", **pad)
            ttk.Entry(rahmen, textvariable=var).grid(row=i * 2, column=1, sticky="ew", **pad)
            leiste_k = ttk.Frame(rahmen)
            leiste_k.grid(row=i * 2, column=2, sticky="w", **pad)
            for k_text, k_befehl in knoepfe:
                ttk.Button(leiste_k, text=k_text, command=k_befehl).pack(side="left", padx=2)
        # Statuszeile unter der Datenbank: zeigt, was in der gewaehlten Datei steckt
        self.lbl_db_info = ttk.Label(rahmen, text="")
        self.lbl_db_info.grid(row=3, column=1, columnspan=2, sticky="w", padx=6)
        # KI-Server: Anzeige plus Dialog
        ttk.Label(rahmen, text="KI-Server:").grid(row=5, column=0, sticky="w", **pad)
        self.lbl_ki = ttk.Label(rahmen, text="")
        self.lbl_ki.grid(row=5, column=1, sticky="w", **pad)
        self.btn_ki = ttk.Button(rahmen, text="Einstellen …", command=self._ki_einstellen)
        self.btn_ki.grid(row=5, column=2, sticky="w", padx=8, pady=3)
        rahmen.columnconfigure(1, weight=1)
        self.var_db.trace_add("write", lambda *_: self._db_info_zeigen())
        # Sicherungsintervall auch waehrend eines Laufs aenderbar
        self.var_backup_alle.trace_add("write", lambda *_: self._backup_alle_geaendert())

        optionen = ttk.Frame(rahmen)
        optionen.grid(row=6, column=0, columnspan=3, sticky="w", **pad)
        ttk.Label(optionen, text="Sicherung alle").pack(side="left")
        ttk.Spinbox(optionen, from_=10, to=100000, increment=100, width=7,
                    textvariable=self.var_backup_alle).pack(side="left", padx=4)
        ttk.Label(optionen, text="neuen Bilder").pack(side="left", padx=(0, 20))
        ttk.Checkbutton(optionen, text="Ruhezustand verhindern, solange der Lauf aktiv ist",
                        variable=self.var_wach).pack(side="left")
        if not metadaten.EXIFTOOL:
            ttk.Label(optionen, text="  ⚠ ExifTool fehlt (brew install exiftool)",
                      foreground="#b35c00").pack(side="left")
        ttk.Label(rahmen, text=f"Einstellungen: {einstellungen.datei()}   |   "
                               f"Python: {sys.executable}",
                  foreground="#777777").grid(row=7, column=0, columnspan=3, sticky="w", **pad)

        # --- Steuerung ---
        leiste = ttk.Frame(self.root)
        leiste.pack(fill="x", padx=10, pady=5)
        self.btn_start = ttk.Button(leiste, text="▶ Start", command=self._start)
        self.btn_pause = ttk.Button(leiste, text="⏸ Pause", command=self._pause, state="disabled")
        self.btn_stopp = ttk.Button(leiste, text="⏹ Stopp", command=self._stopp, state="disabled")
        for b in (self.btn_start, self.btn_pause, self.btn_stopp):
            b.pack(side="left", padx=(0, 6))
        ttk.Button(leiste, text="Protokoll öffnen", command=self._protokoll_oeffnen) \
            .pack(side="right")
        self.lbl_status = ttk.Label(leiste, text="Bereit.", font=("", 12, "bold"))
        self.lbl_status.pack(side="left", padx=15)

        # --- Fortschritt ---
        fort = ttk.LabelFrame(self.root, text="Fortschritt")
        fort.pack(fill="x", padx=10, pady=5)
        self.balken = ttk.Progressbar(fort, mode="determinate")
        self.balken.pack(fill="x", padx=8, pady=(8, 2))
        self.lbl_balken = ttk.Label(fort, text="")
        self.lbl_balken.pack(anchor="w", padx=8)

        werte = ttk.Frame(fort)
        werte.pack(fill="x", padx=8, pady=6)
        self.felder = {}
        anordnung = [("Ordner", 0, 0, 5), ("Datei", 1, 0, 5),
                     ("Neu analysiert", 2, 0, 1), ("Duplikate", 2, 2, 1), ("Fehler", 2, 4, 1),
                     ("Schon erledigt", 3, 0, 1), ("Ø pro Bild", 3, 2, 1), ("Laufzeit", 3, 4, 1),
                     ("Noch offen", 4, 0, 1), ("Restzeit ca.", 4, 2, 1),
                     ("Fertig voraussichtl.", 4, 4, 1),
                     ("Vorschau ergänzt", 5, 0, 1), ("Nur online", 5, 2, 1),
                     ("Entfernt", 5, 4, 1)]
        for name, zeile, spalte, breite in anordnung:
            ttk.Label(werte, text=name + ":").grid(row=zeile, column=spalte, sticky="w", padx=(0, 6))
            lbl = ttk.Label(werte, text="–", font=("", 12))
            lbl.grid(row=zeile, column=spalte + 1, columnspan=breite, sticky="w", padx=(0, 25))
            self.felder[name] = lbl
        for c in (1, 3, 5):
            werte.columnconfigure(c, weight=1)

        # --- Letztes Bild ---
        unten = ttk.PanedWindow(self.root, orient="vertical")
        unten.pack(fill="both", expand=True, padx=10, pady=(5, 10))
        bildrahmen = ttk.LabelFrame(unten, text="Zuletzt analysiert")
        self.lbl_vorschau = ttk.Label(bildrahmen, text="(noch kein Bild)", anchor="center",
                                      width=40)
        self.lbl_vorschau.pack(side="left", padx=8, pady=8)
        self.txt_ergebnis = ScrolledText(bildrahmen, height=12, wrap="word", font=("", 12))
        self.txt_ergebnis.pack(side="left", fill="both", expand=True, padx=(0, 8), pady=8)
        self.txt_ergebnis.tag_configure("titel", font=("", 14, "bold"))
        self.txt_ergebnis.tag_configure("klein", foreground="#666666")
        unten.add(bildrahmen, weight=3)

        # --- Meldungen ---
        lograhmen = ttk.LabelFrame(unten, text="Meldungen")
        self.txt_log = ScrolledText(lograhmen, height=8, wrap="none", font=("Menlo", 11))
        self.txt_log.pack(fill="both", expand=True, padx=8, pady=8)
        unten.add(lograhmen, weight=2)

    # ----------------------------------------------------- Dateiauswahl
    def _ordner_waehlen(self):
        """Dialog zur Auswahl des Bildordners öffnen und Wert übernehmen.

        Rückgabe:
            Keine.
        """
        pfad = filedialog.askdirectory(title="Bildordner wählen")
        if pfad:
            self.var_ordner.set(pfad)

    @staticmethod
    def _db_pruefen(pfad: Path) -> tuple[str, int | None]:
        """Prüfen, was in einer Datenbankdatei steckt (nur lesend).

        Parameter:
            pfad: Datenbankdatei.

        Rückgabe:
            Tupel (Art, Anzahl): ('neu', None) wenn die Datei fehlt,
            ('katalog', Anzahl Einträge) bei einer Katalog-DB,
            ('fremd', None) bei anderen oder unlesbaren Dateien.
        """
        if not pfad.exists():
            return "neu", None
        try:
            db = sqlite3.connect(f"file:{pfad}?mode=ro", uri=True)
            try:
                if not db.execute("SELECT 1 FROM sqlite_master WHERE name = 'bilder'").fetchone():
                    return "fremd", None
                return "katalog", db.execute("SELECT COUNT(*) FROM bilder").fetchone()[0]
            finally:
                db.close()
        except sqlite3.Error:
            return "fremd", None

    def _db_info_zeigen(self):
        """Statuszeile unter dem Datenbankfeld aktualisieren.

        Status-Bereich der Datenbank: zeigt, ob die gewählte Datei eine
        vorhandene Katalog-DB ist (mit aktueller Zahl der Einträge und
        Vorschaubilder und Uhrzeit des Stands), noch nicht existiert oder
        keine Katalog-DB ist. Wird bei Änderung des Pfads und während eines
        Laufs alle paar Sekunden aufgerufen.

        Rückgabe:
            Keine.
        """
        pfad = Path(self.var_db.get().strip()).expanduser()
        if not self.var_db.get().strip():
            self.lbl_db_info.configure(text="", foreground="")
            return
        art, anzahl = self._db_pruefen(pfad)
        if art == "katalog":
            vorschauen = self._vorschauen_zaehlen(pfad)
            text = f"✓ Datenbank vorhanden: {zahl(anzahl)} Einträge"
            text += (f" · {zahl(vorschauen)} Vorschaubilder" if vorschauen is not None
                     else " · noch ohne Vorschaubilder (wird beim Start erweitert)")
            text += f"   (Stand {datetime.now():%H:%M:%S})"
            self.lbl_db_info.configure(text=text, foreground="#2e7d32")
        elif art == "neu":
            self.lbl_db_info.configure(text="Datei existiert noch nicht – wird beim Start angelegt",
                                       foreground="#b35c00")
        else:
            self.lbl_db_info.configure(text="⚠ Diese Datei ist keine Katalog-Datenbank",
                                       foreground="#c00000")

    @staticmethod
    def _vorschauen_zaehlen(pfad: Path) -> int | None:
        """Anzahl der Vorschaubilder in einer Katalog-DB (nur lesend).

        Parameter:
            pfad: Datenbankdatei.

        Rückgabe:
            Anzahl, oder None, wenn die DB (noch) keine Tabelle 'vorschauen' hat.
        """
        try:
            db = sqlite3.connect(f"file:{pfad}?mode=ro", uri=True)
            try:
                if not db.execute("SELECT 1 FROM sqlite_master "
                                  "WHERE name = 'vorschauen'").fetchone():
                    return None
                return db.execute("SELECT COUNT(*) FROM vorschauen").fetchone()[0]
            finally:
                db.close()
        except sqlite3.Error:
            return None

    def _backup_alle_geaendert(self):
        """Geändertes Sicherungsintervall übernehmen – auch während eines Laufs.

        Der laufende Katalogisierer liest 'backup_alle' bei jedem Bild neu;
        der Wert gilt also sofort. Ungültige Zwischeneingaben werden ignoriert.

        Rückgabe:
            Keine.
        """
        try:
            wert = int(self.var_backup_alle.get())
        except (tk.TclError, ValueError):
            return
        if wert < 1:
            return
        if self.katalogisierer:
            self.katalogisierer.backup_alle = wert

    def _startordner(self, wert: str) -> str:
        """Sinnvollen Startordner für Dateidialoge bestimmen.

        Geht vom eingetragenen Pfad so lange nach oben, bis ein vorhandener Ordner
        gefunden ist; ohne Eintrag der Ordner der Standard-Datenbank.

        Parameter:
            wert: Eingetragener Pfad (kann leer sein).

        Rückgabe:
            Pfad eines vorhandenen Ordners als Text.
        """
        p = Path(wert).expanduser() if wert else bildanalyse.STANDARD_DB.parent
        while not p.is_dir() and p != p.parent:
            p = p.parent
        return str(p)

    def _db_oeffnen(self):
        """Vorhandene Katalog-Datenbank per Dialog wählen.

        Nur echte Katalog-Datenbanken werden übernommen; die Auswahl wird sofort
        gespeichert.

        Rückgabe:
            Keine.
        """
        pfad = filedialog.askopenfilename(
            title="Vorhandene Katalog-Datenbank öffnen",
            initialdir=self._startordner(self.var_db.get()),
            filetypes=[("SQLite-Datenbank", "*.db"), ("Alle Dateien", "*")])
        if not pfad:
            return
        art, _ = self._db_pruefen(Path(pfad))
        if art != "katalog":
            messagebox.showerror("Datenbank", "Die gewählte Datei ist keine Katalog-Datenbank.")
            return
        self.var_db.set(pfad)
        self._einstellungen_speichern()

    def _db_neu(self):
        """Pfad für eine neue Katalog-Datenbank per Dialog festlegen.

        Vorhandene Dateien werden nie überschrieben. Die Datei selbst entsteht erst
        beim Start des Laufs.

        Rückgabe:
            Keine.
        """
        pfad = filedialog.asksaveasfilename(
            title="Neue Katalog-Datenbank anlegen",
            initialdir=self._startordner(self.var_db.get()),
            initialfile="Bildkatalog.db", defaultextension=".db",
            filetypes=[("SQLite-Datenbank", "*.db")])
        if not pfad:
            return
        if Path(pfad).exists():
            messagebox.showerror(
                "Datenbank", "Diese Datei existiert bereits und wird nicht überschrieben.\n\n"
                             "Zum Weiterbefüllen bitte „Vorhandene öffnen …“ verwenden.")
            return
        self.var_db.set(pfad)
        self._einstellungen_speichern()

    def _backup_waehlen(self):
        """Ziel der Sicherungskopie per Dialog wählen und speichern.

        Rückgabe:
            Keine.
        """
        pfad = filedialog.asksaveasfilename(
            title="Ziel der Sicherungskopie (z. B. im Dropbox-Ordner)",
            initialdir=self._startordner(self.var_backup.get()),
            initialfile="Bildkatalog_Sicherung.db", defaultextension=".db",
            confirmoverwrite=False, filetypes=[("SQLite-Datenbank", "*.db")])
        if pfad:
            self.var_backup.set(pfad)
            self._einstellungen_speichern()

    def _protokoll_pfad(self) -> Path:
        """Pfad des Protokolls zur gewählten Datenbank.

        Rückgabe:
            <Datenbankordner>/<DB-Name>_protokoll.txt
        """
        db = Path(self.var_db.get()).expanduser()
        return db.with_name(db.stem + "_protokoll.txt")

    def _protokoll_oeffnen(self):
        """Protokolldatei im Standardprogramm öffnen (macOS: open).

        Rückgabe:
            Keine.
        """
        pfad = self._protokoll_pfad()
        if not pfad.exists():
            messagebox.showinfo("Protokoll", f"Noch kein Protokoll vorhanden:\n{pfad}")
            return
        subprocess.run(["open", str(pfad)] if sys.platform == "darwin" else ["xdg-open", str(pfad)])

    # ------------------------------------------------------ Einstellungen
    def _einstellungen_laden(self):
        """Gespeicherte Einstellungen aus der gemeinsamen Einstellungsdatei übernehmen.

        Fehlt die Datei, gelten die Standardwerte (bzw. die migrierten Werte der
        alten gui_einstellungen.json).

        Rückgabe:
            Keine.
        """
        e = einstellungen.laden(bildanalyse.ALT_EINSTELLUNGEN)
        w = e["work"]
        self.var_ordner.set(w.get("ordner", ""))
        self.var_db.set(str(einstellungen.db_pfad(e)))
        self.var_backup.set(w.get("backup", ""))
        self.var_backup_alle.set(w.get("backup_alle", katalog.BACKUP_ALLE))
        self.var_wach.set(w.get("wach", True))
        self.ki = dict(w["ki"])
        bildanalyse.konfigurieren(**self.ki)
        self._ki_zeigen()

    def _einstellungen_speichern(self):
        """Aktuelle Einstellungen in die gemeinsame Einstellungsdatei schreiben.

        Andere Abschnitte (z. B. 'sweep') bleiben unverändert.

        Rückgabe:
            Keine.
        """
        e = einstellungen.laden()
        e["db"] = self.var_db.get().strip()
        try:
            backup_alle = int(self.var_backup_alle.get())
        except (tk.TclError, ValueError):
            backup_alle = katalog.BACKUP_ALLE
        e["work"].update({"ordner": self.var_ordner.get(), "backup": self.var_backup.get(),
                          "backup_alle": backup_alle, "wach": self.var_wach.get(),
                          "ki": dict(self.ki)})
        try:
            einstellungen.speichern(e)
        except OSError as ex:
            self.meldungen.put(f"Einstellungen nicht gespeichert: {ex}")

    # -------------------------------------------------------- KI-Server
    def _ki_zeigen(self):
        """Anzeige des eingestellten KI-Servers aktualisieren.

        Rückgabe:
            Keine.
        """
        modell = self.ki.get("modell") or "⚠ kein Modell gewählt"
        self.lbl_ki.configure(text=f"{modell}   ·   {self.ki.get('url', '')}",
                              foreground="" if self.ki.get("modell") else "#b35c00")

    def _ki_einstellen(self):
        """Dialog „KI-Server“ öffnen und die Werte übernehmen.

        Rückgabe:
            Keine.
        """
        dialog = KiDialog(self.root, self.ki)
        self.root.wait_window(dialog)
        if dialog.ergebnis is None:
            return
        self.ki = dialog.ergebnis
        bildanalyse.konfigurieren(**self.ki)
        self._ki_zeigen()
        self._einstellungen_speichern()

    # ---------------------------------------------------------- Steuerung
    def _start(self):
        """Eingaben prüfen und den Lauf in einem eigenen Thread starten.

        Prüft Bildordner, Datenbank (fremd/neu mit Rückfrage), dass Sicherung und
        Datenbank verschieden sind, und warnt bei einer Datenbank in Dropbox.
        Danach Einstellungen speichern, Knöpfe umschalten, Ruhezustand verhindern.

        Rückgabe:
            Keine.
        """
        ordner = Path(self.var_ordner.get()).expanduser()
        db = Path(self.var_db.get()).expanduser()
        if not ordner.is_dir():
            messagebox.showerror("Bildordner", "Bitte einen vorhandenen Bildordner wählen.")
            return
        if not self.ki.get("modell"):
            messagebox.showerror("KI-Server", "Bitte zuerst KI-Server und Modell einstellen.")
            self._ki_einstellen()
            return
        art, _ = self._db_pruefen(db)
        if art == "fremd":
            messagebox.showerror("Datenbank", "Die gewählte Datenbank-Datei ist keine "
                                              "Katalog-Datenbank. Bitte eine andere wählen.")
            return
        if art == "neu" and not messagebox.askyesno(
                "Neue Datenbank",
                f"Unter diesem Pfad gibt es noch keine Datenbank:\n{db}\n\n"
                f"Neue, leere Datenbank anlegen?\n(Zum Weiterbefüllen einer vorhandenen "
                f"„Nein“ wählen und „Vorhandene öffnen …“ nutzen.)"):
            return
        backup_text = self.var_backup.get().strip()
        if backup_text and Path(backup_text).expanduser().resolve() == db.resolve():
            messagebox.showerror("Sicherung", "Sicherung und Datenbank dürfen nicht dieselbe "
                                              "Datei sein. Bitte ein anderes Sicherungsziel wählen.")
            return
        if "dropbox" in str(db).lower() and not messagebox.askyesno(
                "Datenbank in Dropbox",
                "Die Datenbank liegt in einem Dropbox-Ordner. Empfohlen ist ein lokaler "
                "Ordner plus Sicherungskopie in Dropbox.\n\nTrotzdem fortfahren?"):
            return
        self._einstellungen_speichern()
        backup = self.var_backup.get().strip()

        self.txt_log.delete("1.0", "end")
        self.btn_start.configure(state="disabled")
        self.btn_pause.configure(state="normal", text="⏸ Pause")
        self.btn_stopp.configure(state="normal")
        self.btn_ki.configure(state="disabled")      # Server nicht mitten im Lauf wechseln
        self.lbl_status.configure(text="Läuft …")
        self._wach_halten(True)

        def arbeit():
            """Arbeits-Thread: Katalogisierer anlegen und ausführen.

            Fehler werden als Meldung in die Queue gestellt statt den Thread
            unbemerkt zu beenden.

            Rückgabe:
                Keine.
            """
            try:
                self.katalogisierer = katalog.Katalogisierer(
                    ordner, db, self._protokoll_pfad(),
                    Path(backup) if backup else None,
                    max(1, int(self.var_backup_alle.get())), 0, melden=self.meldungen.put)
                self.katalogisierer.ausfuehren()
            except SystemExit as ex:          # z. B. altes Datenbankformat
                self.meldungen.put(f"FEHLER: {ex}")
            except Exception as ex:
                self.meldungen.put(f"FEHLER: {type(ex).__name__}: {ex}")

        self.thread = threading.Thread(target=arbeit, daemon=True)
        self.thread.start()

    def _pause(self):
        """Pause umschalten (wirkt nach dem aktuellen Bild).

        Rückgabe:
            Keine.
        """
        k = self.katalogisierer
        if not k:
            return
        if k.pause.is_set():
            k.pause.clear()
            self.btn_pause.configure(text="⏸ Pause")
            self.lbl_status.configure(text="Läuft …")
        else:
            k.pause.set()
            self.btn_pause.configure(text="▶ Fortsetzen")
            self.lbl_status.configure(text="Pause nach aktuellem Bild …")

    def _stopp(self):
        """Lauf nach dem aktuellen Bild stoppen.

        Rückgabe:
            Keine.
        """
        k = self.katalogisierer
        if k:
            k.stopp.set()
            k.pause.clear()
            self.btn_pause.configure(state="disabled")
            self.btn_stopp.configure(state="disabled")
            self.lbl_status.configure(text="Stoppe nach aktuellem Bild …")

    def _wach_halten(self, an: bool):
        """Ruhezustand des Mac verhindern bzw. wieder erlauben.

        Startet 'caffeinate -i -w <pid>' (endet spätestens mit diesem Programm)
        oder beendet den laufenden caffeinate-Prozess.

        Parameter:
            an: True = wach halten (nur wenn Option aktiv), False = freigeben.

        Rückgabe:
            Keine.
        """
        if an and self.var_wach.get() and shutil.which("caffeinate") and not self.caffeinate:
            self.caffeinate = subprocess.Popen(["caffeinate", "-i", "-w", str(os.getpid())])
        elif not an and self.caffeinate:
            self.caffeinate.terminate()
            self.caffeinate = None

    def _schliessen(self):
        """Fenster schließen – bei aktivem Lauf erst nach Rückfrage und Stopp.

        Rückgabe:
            Keine.
        """
        if self.thread and self.thread.is_alive():
            if not messagebox.askyesno("Lauf aktiv",
                                       "Der Lauf ist noch aktiv. Nach dem aktuellen Bild "
                                       "stoppen und beenden?\n(Der Fortschritt ist gespeichert.)"):
                return
            self._stopp()
            self.lbl_status.configure(text="Beende nach aktuellem Bild …")
            self._warten_und_beenden()
            return
        self._wach_halten(False)
        self.root.destroy()

    def _warten_und_beenden(self):
        """Warten, bis der Arbeits-Thread beendet ist, dann Fenster schließen.

        Prüft alle 500 ms erneut.

        Rückgabe:
            Keine.
        """
        if self.thread and self.thread.is_alive():
            self.root.after(500, self._warten_und_beenden)
        else:
            self._wach_halten(False)
            self.root.destroy()

    # ------------------------------------------------------- Anzeige
    def _aktualisieren(self):
        """Anzeigeschleife (jede Sekunde im Tk-Hauptthread).

        Übernimmt Meldungen aus der Queue (Anzeige auf 2000 Zeilen begrenzt),
        aktualisiert Kennzahlen und das zuletzt analysierte Bild und setzt die
        Knöpfe zurück, sobald der Lauf beendet ist.

        Rückgabe:
            Keine.
        """
        while not self.meldungen.empty():
            text = self.meldungen.get_nowait()
            self.txt_log.insert("end", text + "\n")
            self.txt_log.see("end")
            zeilen = int(self.txt_log.index("end-1c").split(".")[0])
            if zeilen > 2000:                      # Anzeige begrenzen
                self.txt_log.delete("1.0", f"{zeilen - 2000}.0")

        k = self.katalogisierer
        self._takt = getattr(self, "_takt", 0) + 1
        if self.thread and self._takt % 5 == 0:     # DB-Status alle 5 s waehrend des Laufs
            self._db_info_zeigen()
        if k:
            self._werte_zeigen(k)
            letzes = k.stand.get("letztes_ergebnis")
            if letzes and letzes.get("pfad") != self.letzter_vorschau_pfad:
                self.letzter_vorschau_pfad = letzes.get("pfad")
                self._ergebnis_zeigen(letzes)

        if self.thread and not self.thread.is_alive():
            self.thread = None
            self._db_info_zeigen()
            self._wach_halten(False)
            self.btn_start.configure(state="normal")
            self.btn_ki.configure(state="normal")
            self.btn_pause.configure(state="disabled", text="⏸ Pause")
            self.btn_stopp.configure(state="disabled")
            gestoppt = k and k.stopp.is_set()
            self.lbl_status.configure(text="Gestoppt – Fortsetzen mit Start." if gestoppt
                                      else "Fertig.")
        self.root.after(1000, self._aktualisieren)

    def _werte_zeigen(self, k):
        """Fortschrittsbalken, Kennzahlen und Status aus dem Laufstand anzeigen.

        Parameter:
            k: Laufender Katalogisierer.

        Rückgabe:
            Keine.
        """
        s = k.stand
        f = self.felder
        erledigt = s["neu"] + s["duplikate"] + s["fehler"]
        if s["offen"]:
            self.balken.configure(maximum=s["offen"], value=erledigt)
            anteil = 100 * erledigt / s["offen"]
            self.lbl_balken.configure(
                text=f"{zahl(erledigt)} von {zahl(s['offen'])} offenen Bildern ({anteil:.1f} %)"
                     f"  –  Archiv gesamt: {zahl(s['gesamt'])} Dateien")
        f["Ordner"].configure(text=s["ordner"] or "–")
        f["Datei"].configure(text=s["datei"] or "–")
        f["Neu analysiert"].configure(text=zahl(s["neu"]))
        f["Duplikate"].configure(text=zahl(s["duplikate"]))
        f["Fehler"].configure(text=zahl(s["fehler"]),
                              foreground="#c00000" if s["fehler"] else "")
        f["Schon erledigt"].configure(text=zahl(s["uebersprungen"]))
        f["Vorschau ergänzt"].configure(
            text=zahl(s["vorschauen"]) + (f"  (Fehler: {zahl(s['vorschau_fehler'])})"
                                          if s["vorschau_fehler"] else ""))
        f["Nur online"].configure(text=zahl(s["nur_online"]))
        f["Entfernt"].configure(text=zahl(s["entfernt"]))
        schnitt = s["summe_dauer"] / s["neu"] if s["neu"] else 0
        f["Ø pro Bild"].configure(text=f"{schnitt:.0f} s" if schnitt else "–")
        f["Laufzeit"].configure(text=dauer_text(time.time() - s["start"]))
        offen = k.noch_offen()
        f["Noch offen"].configure(text=zahl(offen))
        rest = schnitt * offen
        f["Restzeit ca."].configure(text=dauer_text(rest))
        f["Fertig voraussichtl."].configure(
            text=(datetime.now() + timedelta(seconds=rest)).strftime("%d.%m.%Y %H:%M")
            if rest else "–")
        if s.get("server_wartet"):
            self.lbl_status.configure(text="⚠ Warte auf KI-Server …")
        elif s.get("pausiert"):
            self.lbl_status.configure(text="⏸ Pausiert – Fortsetzen oder Stopp.")
        elif self.thread and not k.pause.is_set() and not k.stopp.is_set():
            self.lbl_status.configure(text="Läuft …")

    def _ergebnis_zeigen(self, e: dict):
        """Ergebnis des zuletzt analysierten Bildes anzeigen.

        Kurztitel, Pfad, Beschreibung, wichtigste Kategorien, Schlagworte,
        Aufnahmedaten und ggf. Einzelfotos einer Albumseite; dazu die Vorschau.

        Parameter:
            e: Datensatz des Bildes.

        Rückgabe:
            Keine.
        """
        t = self.txt_ergebnis
        t.delete("1.0", "end")
        t.insert("end", (e.get("kurztitel") or "") + "\n", "titel")
        t.insert("end", f"{e.get('relativer_pfad', '')}\n\n", "klein")
        t.insert("end", (e.get("beschreibung") or "") + "\n\n")
        zeilen = [
            ("Motiv", e.get("motiv")), ("Bildart", e.get("bildart")),
            ("Farbe", e.get("farbgebung")), ("Anlass", e.get("anlass")),
            ("Ort / Wetter", f"{e.get('innen_aussen')} / {e.get('wetter')}"),
            ("Zeit", f"{e.get('tageszeit')} / {e.get('jahreszeit')}"),
            ("Personen", f"{e.get('personen_anzahl')}  {', '.join(e.get('personen_merkmale') or [])}"),
            ("Schlagworte", ", ".join(e.get("schlagworte") or [])),
            ("Aufnahme", f"{e.get('aufnahme_datum') or '–'}  {e.get('kamera_modell') or ''}"),
        ]
        for name, wert in zeilen:
            t.insert("end", f"{name}: ", "klein")
            t.insert("end", f"{wert}\n")
        if e.get("einzelbilder"):
            t.insert("end", "\nEinzelne Fotos auf der Seite:\n", "klein")
            for eintrag in e["einzelbilder"]:
                t.insert("end", f"  • {eintrag}\n")
        self._vorschau_laden(e["pfad"])

    def _vorschau_laden(self, schluessel: str):
        """Vorschaubild zu einem Datenbank-Schlüssel laden und anzeigen.

        PDF-Seiten per poppler, Bilder auf dem Mac per 'sips' als PNG (braucht kein
        Pillow, kann auch HEIC), sonst per Pillow. Fehler werden im Vorschaufeld
        und einmalig als Meldung angezeigt.

        Parameter:
            schluessel: Wert aus der Spalte 'pfad'.

        Rückgabe:
            Keine.
        """
        pfad, seite = bildanalyse.quelle_aufloesen(schluessel)
        try:
            if seite is not None:
                with tempfile.TemporaryDirectory() as tmp:
                    png = bildanalyse.pdf_seite_rendern(pfad, seite, tmp, VORSCHAU_PX, "png")
                    self.vorschau_bild = tk.PhotoImage(file=str(png))
            elif shutil.which("sips"):
                with tempfile.TemporaryDirectory() as tmp:
                    ziel = Path(tmp) / "vorschau.png"
                    r = subprocess.run(["sips", "-Z", str(VORSCHAU_PX), "-s", "format", "png",
                                        str(pfad), "--out", str(ziel)],
                                       capture_output=True, text=True, timeout=30)
                    if not ziel.exists():
                        raise RuntimeError(f"sips: {(r.stderr or r.stdout).strip()[:150]}")
                    self.vorschau_bild = tk.PhotoImage(file=str(ziel))  # liest die Datei sofort
            else:
                from PIL import Image, ImageTk
                with Image.open(pfad) as img:
                    img.draft("RGB", (VORSCHAU_PX * 2, VORSCHAU_PX * 2))
                    img = img.convert("RGB")
                    img.thumbnail((VORSCHAU_PX, VORSCHAU_PX))
                    self.vorschau_bild = ImageTk.PhotoImage(img)
            self.lbl_vorschau.configure(image=self.vorschau_bild, text="")
        except Exception as ex:
            meldung = f"{type(ex).__name__}: {ex}"
            self.lbl_vorschau.configure(image="", text=f"(keine Vorschau)\n\n{meldung}",
                                        wraplength=VORSCHAU_PX)
            if not getattr(self, "_vorschau_fehler_gemeldet", False):
                self.meldungen.put(f"Hinweis Vorschau: {meldung}")
                self._vorschau_fehler_gemeldet = True

def main():
    """Programmstart: Tk-Fenster anlegen, Theme setzen, Oberfläche starten.

    Rückgabe:
        Keine.
    """
    root = tk.Tk()
    try:
        ttk.Style().theme_use("aqua" if sys.platform == "darwin" else "clam")
    except tk.TclError:
        pass
    KatalogGUI(root)
    root.mainloop()


if __name__ == "__main__":
    main()
