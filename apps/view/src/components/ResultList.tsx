import { useCallback, useEffect, useRef } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { datumDe } from "../api";
import type { Treffer } from "../types";
import { useTastatur } from "../useTastatur";
import Vorschaubild from "./Vorschaubild";

interface Props {
  treffer: Treffer[];
  auswahl: number | null;
  onAuswahl: (id: number) => void;
}

/** Listenansicht – virtualisiert, mit kleinem Vorschaubild je Zeile. */
export default function ResultList({ treffer, auswahl, onAuswahl }: Props) {
  const liste = useRef<VirtuosoHandle>(null);
  const scrollEl = useRef<HTMLElement | null>(null);

  const index = auswahl === null ? -1 : treffer.findIndex((t) => t.id === auswahl);
  const waehle = useCallback((i: number) => onAuswahl(treffer[i].id), [treffer, onAuswahl]);
  useTastatur({ anzahl: treffer.length, index, waehle, spalten: 1, scroller: () => scrollEl.current });

  useEffect(() => {
    if (index >= 0) liste.current?.scrollIntoView({ index, behavior: "auto" });
  }, [index]);

  return (
    <Virtuoso
      ref={liste}
      className="liste-scroller"
      scrollerRef={(el) => (scrollEl.current = el as HTMLElement | null)}
      data={treffer}
      overscan={300}
      computeItemKey={(_, t) => t.id}
      itemContent={(_, t) => (
        <div
          className={`zeile ${t.id === auswahl ? "zeile--aktiv" : ""}`}
          onClick={() => onAuswahl(t.id)}
        >
          <span className="zeile__vorschau">
            <Vorschaubild pfad={t.pfad} />
          </span>
          <span className="zeile__titel">{t.kurztitel || t.dateiname}</span>
          <span className="zeile__ablage" title={t.pfad}>
            {t.dateiname}
            {t.seite ? ` · Seite ${t.seite}` : ""}
            <span className="leise"> — {ablage(t.archivWurzel, t.ordner)}</span>
          </span>
          <span className="zeile__datum">{datumDe(t.aufnahmeDatum)}</span>
          <span className="zeile__typ">{(t.endung || "").replace(".", "").toUpperCase()}</span>
        </div>
      )}
    />
  );
}

/** Letzter Teil der Archiv-Wurzel + relativer Ordner, z. B. "Globetrotter/2015 Thailand" */
export function ablage(wurzel: string | null, ordner: string | null): string {
  const w = (wurzel || "").split("/").filter(Boolean).pop() || "";
  return [w, ordner].filter(Boolean).join("/");
}
