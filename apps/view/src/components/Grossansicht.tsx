import { useEffect, useRef, useState } from "react";
import { api, zahlDe } from "../api";
import type { Treffer } from "../types";
import { umsetzen, type VorschauStatus } from "../vorschau";
import Vorschaubild from "./Vorschaubild";

interface Props {
  treffer: Treffer;
  /** Position in der Trefferliste (1-basiert) und Anzahl */
  position: [number, number];
  /** −1 / +1: voriges / nächstes Bild */
  onBlaettern: (schritt: number) => void;
  onSchliessen: () => void;
}

type Gross = { art: "laedt" } | VorschauStatus;

/**
 * Großansicht über dem ganzen Fenster (v0.0.6), ähnlich Übersicht im Finder.
 * Zeigt sofort die 640-px-Vorschau und blendet die 2048-px-Fassung ein, sobald sie da ist.
 * Tasten: ←/→ (auch ↑/↓) blättern, Esc oder Leertaste schließen – sie gehen nur an die Großansicht.
 */
export default function Grossansicht({ treffer, position, onBlaettern, onSchliessen }: Props) {
  const [gross, setGross] = useState<Gross>({ art: "laedt" });
  const [geladen, setGeladen] = useState(false);
  const anfrage = useRef(0);

  useEffect(() => {
    const nr = ++anfrage.current;
    setGross({ art: "laedt" });
    setGeladen(false);
    api
      .vorschauGross(treffer.pfad)
      .then((r) => nr === anfrage.current && setGross(umsetzen(r)))
      .catch((m) => nr === anfrage.current && setGross({ art: "fehler", meldung: String(m) }));
  }, [treffer.pfad]);

  // Tasten vor allen anderen abfangen (Raster, Suchfeld, Facetten sollen nichts davon merken)
  const aktionen = useRef({ onBlaettern, onSchliessen });
  aktionen.current = { onBlaettern, onSchliessen };
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      const a = aktionen.current;
      const schritt = { ArrowLeft: -1, ArrowUp: -1, ArrowRight: 1, ArrowDown: 1 }[e.key];
      if (schritt) a.onBlaettern(schritt);
      else if (e.key === "Escape" || e.key === " ") a.onSchliessen();
      else return;
      e.preventDefault();
      e.stopImmediatePropagation();
    };
    window.addEventListener("keydown", h, true);
    return () => window.removeEventListener("keydown", h, true);
  }, []);

  const titel = treffer.kurztitel || treffer.dateiname || "";
  const hinweis =
    gross.art === "laedt"
      ? "Lade große Fassung …"
      : gross.art === "nurOnline"
        ? "Nur online – bitte bei Dropbox anmelden. Angezeigt: 640 px"
        : gross.art === "fehlt"
          ? "Original nicht gefunden. Angezeigt: 640 px"
          : gross.art === "fehler"
            ? `${gross.meldung} – angezeigt: 640 px`
            : null;

  return (
    <div className="grossansicht" role="dialog" aria-label="Großansicht" onClick={onSchliessen}>
      <div className="grossansicht__kopf" onClick={(e) => e.stopPropagation()}>
        <span className="grossansicht__titel">{titel}</span>
        <span className="grossansicht__position">
          {zahlDe(position[0])} / {zahlDe(position[1])}
        </span>
        <button className="grossansicht__zu" onClick={onSchliessen} aria-label="Schließen" title="Schließen (Esc)">
          <svg viewBox="0 0 24 24" aria-hidden>
            <path d="M6 6l12 12M18 6L6 18" />
          </svg>
        </button>
      </div>
      <div className={`grossansicht__bild ${geladen ? "voll" : ""}`}>
        <Vorschaubild pfad={treffer.pfad} />
        {gross.art === "bild" && (
          <img
            className={`grossansicht__voll ${geladen ? "geladen" : ""}`}
            src={gross.url}
            alt=""
            draggable={false}
            onLoad={() => setGeladen(true)}
          />
        )}
      </div>
      <div className="grossansicht__fuss">
        {hinweis ? <span className="grossansicht__hinweis">{hinweis}</span> : <span>2048 px</span>}
        <span>← → blättern · Esc schließen</span>
      </div>
    </div>
  );
}
