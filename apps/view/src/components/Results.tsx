import { useEffect, useState } from "react";
import { zahlDe } from "../api";
import type { Ergebnis } from "../types";
import ResultGrid from "./ResultGrid";
import ResultList from "./ResultList";
import { ergebnisMin } from "../teiler";

interface Props {
  ergebnis: Ergebnis;
  auswahl: number | null;
  onAuswahl: (id: number) => void;
  /** Detailbereich sichtbar (Umschalter in der Kopfzeile); fehlt → kein Umschalter */
  details?: boolean;
  onDetails?: (offen: boolean) => void;
  /** meldet, wie breit das Ergebnis mindestens sein muss (eine Kachel bzw. Liste) – für den Schieber */
  onMindestbreite?: (px: number) => void;
}

type Ansicht = "raster" | "liste";

// kleine Vorlieben des Betrachters – Zugriff abgesichert
function lese<T>(k: string, standard: T): T {
  try {
    const v = localStorage.getItem(k);
    return v === null ? standard : (JSON.parse(v) as T);
  } catch {
    return standard;
  }
}
function schreibe(k: string, v: unknown) {
  try {
    localStorage.setItem(k, JSON.stringify(v));
  } catch {
    /* egal */
  }
}

/** Kopfzeile (Anzahl, Ansicht, Kachelgröße) + Raster oder Liste. */
export default function Results({ ergebnis, auswahl, onAuswahl, details, onDetails, onMindestbreite }: Props) {
  const [ansicht, setAnsicht] = useState<Ansicht>(() => lese("view.ansicht", "raster"));
  const [kachel, setKachel] = useState<number>(() => lese("view.kachel", 180));
  useEffect(() => schreibe("view.ansicht", ansicht), [ansicht]);
  useEffect(() => schreibe("view.kachel", kachel), [kachel]);
  useEffect(() => onMindestbreite?.(ergebnisMin(ansicht, kachel)), [ansicht, kachel, onMindestbreite]);

  const { gesamt, treffer, dauerMs } = ergebnis;
  if (gesamt === 0) return <div className="hinweis">Keine Treffer.</div>;

  return (
    <section className="ergebnis">
      <div className="ergebnis__kopf">
        <span>
          {zahlDe(gesamt)} {gesamt === 1 ? "Bild" : "Bilder"}
          {treffer.length < gesamt && <> · die ersten {zahlDe(treffer.length)} angezeigt</>}
          <span className="leise"> · {dauerMs} ms</span>
        </span>
        <span className="ergebnis__werkzeuge">
          {ansicht === "raster" && (
            <input
              className="regler"
              type="range"
              min={110}
              max={340}
              step={10}
              value={kachel}
              onChange={(e) => setKachel(+e.target.value)}
              title="Kachelgröße"
            />
          )}
          <span className="umschalter">
            <button
              className={ansicht === "raster" ? "aktiv" : ""}
              onClick={() => setAnsicht("raster")}
              title="Raster"
            >
              <svg viewBox="0 0 24 24" aria-hidden>
                <rect x="4" y="4" width="7" height="7" rx="1.5" />
                <rect x="13" y="4" width="7" height="7" rx="1.5" />
                <rect x="4" y="13" width="7" height="7" rx="1.5" />
                <rect x="13" y="13" width="7" height="7" rx="1.5" />
              </svg>
            </button>
            <button
              className={ansicht === "liste" ? "aktiv" : ""}
              onClick={() => setAnsicht("liste")}
              title="Liste"
            >
              <svg viewBox="0 0 24 24" aria-hidden>
                <path d="M4 6h16M4 12h16M4 18h16" />
              </svg>
            </button>
          </span>
          {onDetails && (
            <button
              className={`umschalter__einzeln ${details ? "aktiv" : ""}`}
              onClick={() => onDetails(!details)}
              title={details ? "Details ausblenden" : "Details einblenden"}
              aria-pressed={!!details}
            >
              <svg viewBox="0 0 24 24" aria-hidden>
                <rect x="3.5" y="4.5" width="17" height="15" rx="2" />
                <path d="M14.5 4.5v15" />
              </svg>
            </button>
          )}
        </span>
      </div>
      {ansicht === "raster" ? (
        <ResultGrid treffer={treffer} auswahl={auswahl} onAuswahl={onAuswahl} kachel={kachel} />
      ) : (
        <ResultList treffer={treffer} auswahl={auswahl} onAuswahl={onAuswahl} />
      )}
    </section>
  );
}
