import { useCallback, useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { alleNeuLaden } from "./vorschau";
import { api, zahlDe } from "./api";
import { LEERER_FILTER, type DbInfo, type Ergebnis, type Facette, type SuchFilter } from "./types";
import SearchPanel from "./components/SearchPanel";
import Results from "./components/Results";
import DbMenu from "./components/DbMenu";
import Detailbereich from "./components/Detailbereich";
import Teiler from "./components/Teiler";
import Grossansicht from "./components/Grossansicht";
import EinstellungenDialog from "./components/EinstellungenDialog";
import { begrenzen, DETAIL_STANDARD } from "./teiler";

type Zustand =
  | { art: "laedt" }
  | { art: "ohneDb"; meldung: string }
  | { art: "bereit"; info: DbInfo };

/** true, wenn kein Suchkriterium gesetzt ist (Gegenstück zu SuchFilter::ist_leer in Rust). */
export function istLeer(f: SuchFilter) {
  return (
    !f.freitext.trim() &&
    !f.name.trim() &&
    !f.ordner.trim() &&
    !f.datumVon.trim() &&
    !f.datumBis.trim() &&
    f.endungen.length === 0 &&
    Object.values(f.facetten ?? {}).every((w) => w.length === 0)
  );
}

/** Detailbereich ein/aus – gemerkt (Zugriff abgesichert) */
function detailsGemerkt(): boolean {
  try {
    return localStorage.getItem("view.details") !== "false";
  } catch {
    return true;
  }
}

function breiteGemerkt(): number {
  try {
    const n = Number(localStorage.getItem("view.detailbreite"));
    return Number.isFinite(n) && n > 0 ? n : DETAIL_STANDARD;
  } catch {
    return DETAIL_STANDARD;
  }
}

export default function App() {
  const [zustand, setZustand] = useState<Zustand>({ art: "laedt" });
  const [filter, setFilter] = useState<SuchFilter>(LEERER_FILTER);
  const [ergebnis, setErgebnis] = useState<Ergebnis | null>(null);
  const [fehler, setFehler] = useState<string | null>(null);
  const [auswahl, setAuswahl] = useState<number | null>(null);
  const [facetten, setFacetten] = useState<Facette[]>([]);
  const [details, setDetails] = useState<boolean>(detailsGemerkt);
  const [grossOffen, setGrossOffen] = useState(false);
  const [einstOffen, setEinstOffen] = useState(false);
  const einstellungenOeffnen = useCallback(() => {
    setGrossOffen(false);
    setEinstOffen(true);
  }, []);
  /** zählt hoch, wenn die DB neu geladen wurde → Suche, Facetten, Details frisch holen */
  const [stand, setStand] = useState(0);
  const [meldung, setMeldung] = useState<string | null>(null);
  const [detailBreite, setDetailBreite] = useState<number>(breiteGemerkt);
  const [ergebnisMinimum, setErgebnisMinimum] = useState(0);
  const [bereichBreite, setBereichBreite] = useState(0);
  const bereich = useRef<HTMLDivElement | null>(null);
  const anfrage = useRef(0);
  const facettenAnfrage = useRef(0);

  useEffect(() => {
    api
      .start()
      .then((info) => setZustand({ art: "bereit", info }))
      .catch((m) => setZustand({ art: "ohneDb", meldung: String(m) }));
  }, []);

  // Dropbox hat die Sicherung ersetzt (oder „Neu laden“): neue Info, alles frisch holen, Auswahl bleibt
  useEffect(() => {
    let ab: (() => void) | undefined;
    let aus = false;
    listen<DbInfo>("db-neu", (e) => {
      setZustand({ art: "bereit", info: e.payload });
      setStand((n) => n + 1);
      alleNeuLaden();
      setMeldung(`Katalog aktualisiert · ${zahlDe(e.payload.bilder)} Bilder`);
    })
      .then((f) => (aus ? f() : (ab = f)))
      .catch(() => {});
    return () => {
      aus = true;
      ab?.();
    };
  }, []);

  useEffect(() => {
    if (!meldung) return;
    const t = window.setTimeout(() => setMeldung(null), 4000);
    return () => window.clearTimeout(t);
  }, [meldung]);

  // „Vorschau-Cache leeren“ (App-Menü oder Einstellungen) und neue Speicherorte/Dropbox-Schlüssel (v0.3):
  // alle Vorschauen neu anfordern. „einstellungen-oeffnen“ kommt aus dem App-Menü (⌘,).
  useEffect(() => {
    const ab: (() => void)[] = [];
    let aus = false;
    const hoeren = (name: string, f: () => void) =>
      listen(name, f)
        .then((weg) => (aus ? weg() : ab.push(weg)))
        .catch(() => {});
    hoeren("cache-geleert", () => alleNeuLaden());
    hoeren("speicher-neu", () => alleNeuLaden());
    hoeren("einstellungen-oeffnen", einstellungenOeffnen);
    return () => {
      aus = true;
      ab.forEach((f) => f());
    };
  }, [einstellungenOeffnen]);

  const dbWaehlen = useCallback(async () => {
    const pfad = await open({
      title: "Bildkatalog-Datenbank auswählen",
      multiple: false,
      directory: false,
      filters: [{ name: "SQLite-Datenbank", extensions: ["db", "sqlite", "sqlite3"] }],
    });
    if (typeof pfad !== "string") return;
    try {
      const info = await api.dbSetzen(pfad);
      setZustand({ art: "bereit", info });
      setErgebnis(null);
      setAuswahl(null);
    } catch (m) {
      setZustand({ art: "ohneDb", meldung: String(m) });
    }
  }, []);

  // Suche mit kurzer Verzögerung; nur die jüngste Antwort zählt.
  useEffect(() => {
    if (zustand.art !== "bereit") return;
    if (istLeer(filter)) {
      anfrage.current++;
      setErgebnis(null);
      setFehler(null);
      return;
    }
    const nr = ++anfrage.current;
    const t = window.setTimeout(() => {
      api
        .suchen(filter)
        .then((e) => {
          if (nr !== anfrage.current) return;
          setErgebnis(e);
          setFehler(null);
          setAuswahl((a) => (a !== null && e.treffer.some((t) => t.id === a) ? a : null));
        })
        .catch((m) => nr === anfrage.current && setFehler(String(m)));
    }, 220);
    return () => window.clearTimeout(t);
  }, [filter, zustand.art, stand]);

  // Facetten mit Anzahl – auch ohne Suche (Stöbern im ganzen Katalog); nur die jüngste Antwort zählt.
  useEffect(() => {
    if (zustand.art !== "bereit") return;
    const nr = ++facettenAnfrage.current;
    const t = window.setTimeout(() => {
      api
        .facetten(filter)
        .then((f) => nr === facettenAnfrage.current && setFacetten(f))
        .catch(() => nr === facettenAnfrage.current && setFacetten([]));
    }, 220);
    return () => window.clearTimeout(t);
  }, [filter, zustand]);

  const hatSuche = ergebnis !== null || fehler !== null;

  const detailsSetzen = useCallback((offen: boolean) => {
    setDetails(offen);
    try {
      localStorage.setItem("view.details", String(offen));
    } catch {
      /* egal */
    }
  }, []);

  // Breite des Arbeitsbereichs beobachten (Fenstergröße) – begrenzt den Schieber
  const bereichSetzen = useCallback((el: HTMLDivElement | null) => {
    bereich.current = el;
    if (el) setBereichBreite(el.clientWidth);
  }, []);
  useEffect(() => {
    const el = bereich.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setBereichBreite(el.clientWidth));
    ro.observe(el);
    return () => ro.disconnect();
  }, [ergebnis !== null]);

  const breiteMerken = useCallback((b: number) => {
    try {
      localStorage.setItem("view.detailbreite", String(b));
    } catch {
      /* egal */
    }
  }, []);

  const index = ergebnis && auswahl !== null ? ergebnis.treffer.findIndex((t) => t.id === auswahl) : -1;

  const auswahlDa = index >= 0;
  const anzahl = ergebnis?.treffer.length ?? 0;
  useEffect(() => {
    if (!auswahlDa) setGrossOffen(false);
  }, [auswahlDa]);

  // Leertaste öffnet die Großansicht – nicht in Eingabefeldern (Suchzeile bleibt unberührt)
  useEffect(() => {
    if (!auswahlDa || grossOffen) return;
    const h = (e: KeyboardEvent) => {
      if (e.key !== " " || e.metaKey || e.ctrlKey || e.altKey || e.repeat) return;
      const a = document.activeElement as HTMLElement | null;
      if (a && (a instanceof HTMLInputElement || a instanceof HTMLTextAreaElement || a instanceof HTMLSelectElement || a.isContentEditable)) return;
      e.preventDefault();
      setGrossOffen(true);
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [auswahlDa, grossOffen]);

  // Esc außerhalb von Eingabefeldern hebt die Auswahl auf (Detailbereich schließt) –
  // nicht, solange Facetten-Werte aufgeklappt sind (dort schließt Esc zuerst den Aufklapper)
  useEffect(() => {
    if (!auswahlDa || grossOffen) return;
    const h = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.metaKey || e.ctrlKey || e.altKey) return;
      if (document.activeElement instanceof HTMLInputElement || document.querySelector(".facette__werte")) return;
      setAuswahl(null);
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [auswahlDa, grossOffen]);

  const blaettern = useCallback(
    (schritt: number) => {
      if (!ergebnis || index < 0) return;
      const ziel = Math.max(0, Math.min(anzahl - 1, index + schritt));
      setAuswahl(ergebnis.treffer[ziel].id);
    },
    [ergebnis, index, anzahl],
  );

  return (
    <div className={`app ${hatSuche ? "app--suche" : "app--leer"}`}>
      <div className="titelleiste" data-tauri-drag-region>
        <span className="titelleiste__name" data-tauri-drag-region>
          Bilderwerk View
        </span>
        {zustand.art === "bereit" && (
          <DbMenu info={zustand.info} onWechseln={dbWaehlen} onEinstellungen={einstellungenOeffnen} />
        )}
      </div>

      {zustand.art === "ohneDb" && (
        <div className="einrichtung">
          <h1>Bilderwerk View</h1>
          <p>{zustand.meldung}</p>
          <div className="einrichtung__knoepfe">
            <button className="knopf knopf--primaer" onClick={dbWaehlen}>
              Bildkatalog auswählen…
            </button>
            <button className="knopf" onClick={einstellungenOeffnen}>
              Einstellungen…
            </button>
          </div>
        </div>
      )}

      {einstOffen && (
        <EinstellungenDialog
          db={zustand.art === "bereit" ? zustand.info : null}
          onDbWechseln={dbWaehlen}
          onSchliessen={() => setEinstOffen(false)}
        />
      )}

      {zustand.art === "bereit" && (
        <>
          <SearchPanel filter={filter} onChange={setFilter} kompakt={hatSuche} facetten={facetten} />
          {fehler && <div className="hinweis hinweis--fehler">{fehler}</div>}
          {ergebnis && !fehler && (
            <div className="arbeitsbereich" ref={bereichSetzen}>
              <Results
                ergebnis={ergebnis}
                auswahl={auswahl}
                onAuswahl={setAuswahl}
                details={details}
                onDetails={detailsSetzen}
                onMindestbreite={setErgebnisMinimum}
              />
              {details && index >= 0 && (
                <>
                  <Teiler
                    bereich={() => bereich.current}
                    onZiehen={(w) => setDetailBreite(begrenzen(w, bereichBreite, ergebnisMinimum))}
                    onFertig={() => breiteMerken(begrenzen(detailBreite, bereichBreite, ergebnisMinimum))}
                    onZuruecksetzen={() => {
                      setDetailBreite(DETAIL_STANDARD);
                      breiteMerken(DETAIL_STANDARD);
                    }}
                  />
                  <Detailbereich
                    treffer={ergebnis.treffer[index]}
                    position={[index + 1, ergebnis.gesamt]}
                    breite={bereichBreite > 0 ? begrenzen(detailBreite, bereichBreite, ergebnisMinimum) : undefined}
                    onGross={() => setGrossOffen(true)}
                    stand={stand}
                  />
                </>
              )}
            </div>
          )}
          {meldung && <div className="meldung-oben" role="status">{meldung}</div>}
          {grossOffen && ergebnis && index >= 0 && (
            <Grossansicht
              treffer={ergebnis.treffer[index]}
              position={[index + 1, ergebnis.gesamt]}
              onBlaettern={blaettern}
              onSchliessen={() => setGrossOffen(false)}
            />
          )}
        </>
      )}
    </div>
  );
}
