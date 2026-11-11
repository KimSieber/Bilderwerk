"""Bilderwerk Sweep – Dubletten im Bildkatalog sichten und einzeln löschen.

Start:  python3 sweep.py [Pfad/zur/Bildkatalog.db]
Ohne Angabe: DB aus apps/work/gui_einstellungen.json bzw. Dateiauswahl.

Arbeitet auf der LIVE-DB (Mac mini, parallel zum Work) – nie auf der Sicherung.
Links: Gruppen identischer Dateien. Rechts: Vorschau, Name, Ablageort, Auflösung,
Größe und je Datei ein Lösch-Button (mit Bestätigung). PDFs nur als Hinweis.
Jede Löschung wird in sweep_protokoll.csv neben der DB protokolliert.
"""
from __future__ import annotations

import io
import subprocess
import sys
import tkinter as tk
from pathlib import Path
from tkinter import filedialog, messagebox, ttk

from PIL import Image, ImageTk

import daten

KARTE_PX = 480  # Anzeigegröße der Vorschau (aus 640-px-Vorschau verkleinert)


class SweepApp(tk.Tk):
    def __init__(self, db_pfad: Path):
        super().__init__()
        self.title(f"Bilderwerk Sweep – {db_pfad}")
        self.geometry("1500x900")
        self.db_pfad = db_pfad
        self.db = daten.verbinden(db_pfad)
        self.protokoll = daten.protokoll_pfad(db_pfad)
        self.gruppen: list[daten.Gruppe] = []
        self.bilder: list[ImageTk.PhotoImage] = []   # Referenzen halten
        self.pdf_zeigen = tk.BooleanVar(value=True)
        self._oberflaeche()
        self.laden()

    # -- Aufbau --------------------------------------------------------------
    def _oberflaeche(self):
        leiste = ttk.Frame(self, padding=6)
        leiste.pack(fill="x")
        ttk.Button(leiste, text="Neu laden", command=self.laden).pack(side="left")
        ttk.Checkbutton(leiste, text="PDFs anzeigen", variable=self.pdf_zeigen,
                        command=self._liste_fuellen).pack(side="left", padx=10)
        self.status = ttk.Label(leiste, text="")
        self.status.pack(side="left", padx=10)
        ttk.Label(leiste, text=f"Protokoll: {self.protokoll}", foreground="gray").pack(side="right")

        teil = ttk.PanedWindow(self, orient="horizontal")
        teil.pack(fill="both", expand=True)

        links = ttk.Frame(teil)
        spalten = ("anzahl", "typ", "name", "groesse")
        self.liste = ttk.Treeview(links, columns=spalten, show="headings", selectmode="browse")
        for s, t, b in (("anzahl", "#", 40), ("typ", "Typ", 50), ("name", "Name(n)", 260),
                        ("groesse", "Größe", 80)):
            self.liste.heading(s, text=t)
            self.liste.column(s, width=b, stretch=(s == "name"))
        sb = ttk.Scrollbar(links, orient="vertical", command=self.liste.yview)
        self.liste.configure(yscrollcommand=sb.set)
        self.liste.pack(side="left", fill="both", expand=True)
        sb.pack(side="right", fill="y")
        self.liste.bind("<<TreeviewSelect>>", lambda _e: self._gruppe_zeigen())
        teil.add(links, weight=1)

        rechts = ttk.Frame(teil)
        self.flaeche = tk.Canvas(rechts, highlightthickness=0, yscrollincrement=20)
        sb2 = ttk.Scrollbar(rechts, orient="vertical", command=self.flaeche.yview)
        self.flaeche.configure(yscrollcommand=sb2.set)
        sb2.pack(side="right", fill="y")
        self.flaeche.pack(side="left", fill="both", expand=True)
        self.inhalt = ttk.Frame(self.flaeche, padding=8)
        fenster = self.flaeche.create_window((0, 0), window=self.inhalt, anchor="nw")
        self.inhalt.bind("<Configure>",
                         lambda _e: self.flaeche.configure(scrollregion=self.flaeche.bbox("all")))
        # Inhalt immer so breit wie der sichtbare Bereich
        self.flaeche.bind("<Configure>",
                          lambda e: self.flaeche.itemconfigure(fenster, width=e.width))
        # Mausrad nur über dem rechten Bereich (die Liste scrollt selbst)
        # (Enter/Leave taugt nicht: Tk meldet Leave schon beim Wechsel auf ein Kind-Widget)
        self.rechts = rechts
        self._rest = 0.0
        self.bind_all("<MouseWheel>", self._rad)
        try:                                   # Tk 9 (macOS): Touchpad sendet eigene Ereignisse
            self.bind_all("<TouchpadScroll>", self._touchpad)
        except tk.TclError:
            pass
        teil.add(rechts, weight=3)

    def _ueber_rechts(self, e) -> bool:
        w = self.winfo_containing(e.x_root, e.y_root)
        return bool(w) and str(w).startswith(str(self.rechts))

    def _rad(self, e):
        if e.delta and self._ueber_rechts(e):
            self.flaeche.yview_scroll(-1 if e.delta > 0 else 1, "units")

    def _touchpad(self, e):
        if not self._ueber_rechts(e):
            return
        try:
            _dx, dy = self.tk.splitlist(self.tk.call("tk::PreciseScrollDeltas", e.delta))
        except tk.TclError:
            return
        self._rest -= float(dy) / 5
        schritte = int(self._rest)
        if schritte:
            self._rest -= schritte
            self.flaeche.yview_scroll(schritte, "units")

    # -- Daten ---------------------------------------------------------------
    def laden(self):
        self.config(cursor="watch")
        self.update_idletasks()
        try:
            self.gruppen = daten.gruppen_laden(self.db)
        finally:
            self.config(cursor="")
        self._liste_fuellen()

    def _sichtbar(self) -> list[daten.Gruppe]:
        return [g for g in self.gruppen if self.pdf_zeigen.get() or not g.ist_pdf]

    def _liste_fuellen(self, auswahl: int | None = None):
        self.liste.delete(*self.liste.get_children())
        sichtbar = self._sichtbar()
        for i, g in enumerate(sichtbar):
            self.liste.insert("", "end", iid=str(i), values=self._zeile(g))
        bilder = sum(1 for g in self.gruppen if not g.ist_pdf)
        self.status.config(text=f"{len(self.gruppen)} Gruppen ({bilder} Bild, "
                                f"{len(self.gruppen) - bilder} PDF)")
        if sichtbar:
            i = min(auswahl or 0, len(sichtbar) - 1)
            self.liste.selection_set(str(i))
            self.liste.see(str(i))
        else:
            self._leeren()

    @staticmethod
    def _zeile(g: daten.Gruppe) -> tuple:
        """Listenzeile; unterschiedliche Dateinamen der Gruppe werden alle gezeigt."""
        namen = list(dict.fromkeys(d.name for d in g.dateien))
        return (len(g.dateien), "PDF" if g.ist_pdf else "Bild", "  ·  ".join(namen),
                daten.groesse_text(g.dateien[0].groesse))

    def _aktuelle(self) -> tuple[int, daten.Gruppe] | None:
        sel = self.liste.selection()
        if not sel:
            return None
        i = int(sel[0])
        return i, self._sichtbar()[i]

    # -- Anzeige -------------------------------------------------------------
    def _leeren(self):
        for w in self.inhalt.winfo_children():
            w.destroy()
        self.bilder.clear()

    def _gruppe_zeigen(self):
        a = self._aktuelle()
        if not a:
            return
        _, g = a
        self._leeren()
        if g.ist_pdf:
            ttk.Label(self.inhalt, foreground="#b35c00",
                      text="PDF – nur Hinweis. Bitte die Dateien im Finder prüfen und dort "
                           "löschen; der nächste vollständige Work-Lauf räumt die DB auf.",
                      wraplength=900).pack(anchor="w", pady=(0, 8))
        for d in g.dateien:
            self._karte(g, d).pack(fill="x", padx=8, pady=8)
        self.update_idletasks()
        self.flaeche.configure(scrollregion=self.flaeche.bbox("all"))
        self.flaeche.yview_moveto(0)

    def _karte(self, g: daten.Gruppe, d: daten.Datei) -> ttk.Frame:
        k = ttk.Frame(self.inhalt, padding=6, relief="groove")
        jpeg, hinweis = daten.vorschau(self.db, d)
        if jpeg:
            img = Image.open(io.BytesIO(jpeg))
            img.thumbnail((KARTE_PX, KARTE_PX))
            foto = ImageTk.PhotoImage(img)
            self.bilder.append(foto)
            tk.Label(k, image=foto).pack(anchor="w")
        else:
            tk.Label(k, text=hinweis, width=60, height=8).pack(anchor="w")
        aufl = f"{d.breite} × {d.hoehe} px" if d.breite and d.hoehe else "–"
        info = [("Name", d.name), ("Ablage", d.ordner), ("Auflösung", aufl),
                ("Größe", daten.groesse_text(d.groesse))]
        if d.ist_pdf:
            info.append(("Seiten", str(d.seiten or len(d.ids))))
        t = ttk.Frame(k)
        t.pack(fill="x", pady=4)
        for r, (a, b) in enumerate(info):
            ttk.Label(t, text=a + ":", foreground="gray").grid(row=r, column=0, sticky="nw")
            ttk.Label(t, text=b, wraplength=900).grid(row=r, column=1, sticky="w")
        knoepfe = ttk.Frame(k)
        knoepfe.pack(fill="x")
        ttk.Button(knoepfe, text="Im Finder zeigen",
                   command=lambda: subprocess.run(["open", "-R", d.pfad])).pack(side="left")
        if not d.ist_pdf:
            ttk.Button(knoepfe, text="Löschen …",
                       command=lambda: self._loeschen(g, d)).pack(side="right")
        return k

    # -- Aktion --------------------------------------------------------------
    def _loeschen(self, g: daten.Gruppe, d: daten.Datei):
        rest = "\n".join(f"  • {x.pfad}" for x in g.dateien if x is not d)
        if not messagebox.askyesno(
                "Datei löschen?",
                f"Diese Datei wird vom Laufwerk und aus der Datenbank gelöscht:\n\n"
                f"{d.pfad}\n\nEs bleibt erhalten:\n{rest}",
                icon="warning", default="no"):
            return
        a = self._aktuelle()
        try:
            daten.loeschen(self.db, g, d, self.protokoll)
        except Exception as ex:
            messagebox.showerror("Löschen fehlgeschlagen", f"{type(ex).__name__}: {ex}")
            return
        if len(g.dateien) < 2:
            self.gruppen.remove(g)
            self._liste_fuellen(a[0] if a else 0)
        else:
            if a:
                self.liste.item(str(a[0]), values=self._zeile(g))
            self._gruppe_zeigen()


def main():
    pfad = Path(sys.argv[1]).expanduser() if len(sys.argv) > 1 else daten.standard_db()
    if not pfad or not pfad.exists():
        wurzel = tk.Tk()
        wurzel.withdraw()
        gewaehlt = filedialog.askopenfilename(title="Live-Katalog-DB wählen",
                                              filetypes=[("SQLite", "*.db"), ("Alle", "*")])
        wurzel.destroy()
        if not gewaehlt:
            return
        pfad = Path(gewaehlt)
    SweepApp(pfad).mainloop()


if __name__ == "__main__":
    main()
