import { useEffect, useRef, useState } from "react";
import { api, zahlDe } from "../api";
import type { DbInfo } from "../types";

interface Props {
  info: DbInfo;
  onWechseln: () => void;
  /** Einstellungsdialog öffnen (Cache, Speicherorte, Dropbox – seit v0.3 dort) */
  onEinstellungen: () => void;
}

/** Kleines Menü oben rechts: Stand der Datenbank-Sicherung, Neu laden, Wechsel der Datei, Einstellungen. */
export default function DbMenu({ info, onWechseln, onEinstellungen }: Props) {
  const [offen, setOffen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const [laedt, setLaedt] = useState(false);
  const [neuFehler, setNeuFehler] = useState<string | null>(null);

  // Neu laden: die App aktualisiert sich über das Ereignis „db-neu“
  const neuLaden = async () => {
    setLaedt(true);
    setNeuFehler(null);
    try {
      await api.dbNeuLaden();
    } catch (e) {
      setNeuFehler(String(e));
    } finally {
      setLaedt(false);
    }
  };

  useEffect(() => {
    if (!offen) return;
    const h = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOffen(false);
    };
    window.addEventListener("mousedown", h);
    return () => window.removeEventListener("mousedown", h);
  }, [offen]);

  const stand = info.geaendert
    ? new Date(info.geaendert * 1000).toLocaleString("de-DE", { dateStyle: "medium", timeStyle: "short" })
    : "unbekannt";

  return (
    <div className="dbmenu" ref={ref}>
      <button className="dbmenu__knopf" onClick={() => setOffen(!offen)} title="Datenbank">
        <svg viewBox="0 0 24 24" aria-hidden>
          <ellipse cx="12" cy="6" rx="7" ry="2.5" />
          <path d="M5 6v12c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5V6" />
          <path d="M5 12c0 1.4 3.1 2.5 7 2.5s7-1.1 7-2.5" />
        </svg>
      </button>
      {offen && (
        <div className="dbmenu__panel">
          <div className="dbmenu__zeile">
            <span className="leise">Bilder</span>
            <span>{zahlDe(info.bilder)}</span>
          </div>
          <div className="dbmenu__zeile">
            <span className="leise">Duplikate</span>
            <span>{zahlDe(info.duplikate)}</span>
          </div>
          <div className="dbmenu__zeile">
            <span className="leise">Stand</span>
            <span>{stand}</span>
          </div>
          <div className="dbmenu__zeile">
            <span className="leise">Vorschauen in der DB</span>
            <span>{info.vorschauen === null ? "keine" : zahlDe(info.vorschauen)}</span>
          </div>
          <div className="dbmenu__pfad" title={info.pfad}>
            {info.pfad}
          </div>
          {neuFehler && <div className="dbmenu__fehler">{neuFehler}</div>}
          <div className="dbmenu__knoepfe">
            <button className="knopf" onClick={neuLaden} disabled={laedt} title="Neu laden (⌘R)">
              {laedt ? "Lädt …" : "Neu laden"}
            </button>
            <button
              className="knopf"
              onClick={() => {
                setOffen(false);
                onWechseln();
              }}
            >
              Andere Datenbank…
            </button>
          </div>
          <div className="dbmenu__einst">
            <button
              className="knopf"
              onClick={() => {
                setOffen(false);
                onEinstellungen();
              }}
              title="Einstellungen (⌘,)"
            >
              Einstellungen…
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
