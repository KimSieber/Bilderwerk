import { useCallback, useEffect, useRef, useState } from "react";
import { VirtuosoGrid, type VirtuosoGridHandle } from "react-virtuoso";
import { datumDe } from "../api";
import type { Treffer } from "../types";
import { useTastatur } from "../useTastatur";
import Vorschaubild from "./Vorschaubild";

interface Props {
  treffer: Treffer[];
  auswahl: number | null;
  onAuswahl: (id: number) => void;
  kachel: number; // Mindestbreite einer Kachel in px
}

const ABSTAND = 14;
const RAND = 6;

export default function ResultGrid({ treffer, auswahl, onAuswahl, kachel }: Props) {
  const grid = useRef<VirtuosoGridHandle>(null);
  const scrollEl = useRef<HTMLElement | null>(null);
  const bereich = useRef({ start: 0, ende: 0 });
  const [breite, setBreite] = useState(0);

  // Spaltenzahl wie im CSS: repeat(auto-fill, minmax(kachel, 1fr))
  const spalten = Math.max(1, Math.floor((breite - 2 * RAND + ABSTAND) / (kachel + ABSTAND)));

  useEffect(() => {
    const el = scrollEl.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setBreite(el.clientWidth));
    ro.observe(el);
    setBreite(el.clientWidth);
    return () => ro.disconnect();
  }, []);

  const index = auswahl === null ? -1 : treffer.findIndex((t) => t.id === auswahl);
  const waehle = useCallback((i: number) => onAuswahl(treffer[i].id), [treffer, onAuswahl]);
  useTastatur({ anzahl: treffer.length, index, waehle, spalten, scroller: () => scrollEl.current });

  // Auswahl sichtbar halten – nur scrollen, wenn sie außerhalb des sichtbaren Bereichs liegt
  useEffect(() => {
    if (index < 0) return;
    const { start, ende } = bereich.current;
    if (index < start + spalten || index > ende - spalten) {
      grid.current?.scrollToIndex({ index, align: index < start + spalten ? "start" : "end" });
    }
  }, [index, spalten]);

  return (
    <VirtuosoGrid
      ref={grid}
      className="raster-scroller"
      scrollerRef={(el) => {
        scrollEl.current = el as HTMLElement | null;
        if (el) setBreite((el as HTMLElement).clientWidth);
      }}
      style={{ ["--kachel" as string]: `${kachel}px` }}
      totalCount={treffer.length}
      overscan={400}
      listClassName="raster"
      computeItemKey={(i) => treffer[i].id}
      rangeChanged={(r) => (bereich.current = { start: r.startIndex, ende: r.endIndex })}
      itemContent={(i) => {
        const t = treffer[i];
        return (
          <div
            className={`kachel ${t.id === auswahl ? "kachel--aktiv" : ""}`}
            onClick={() => onAuswahl(t.id)}
            title={t.pfad}
          >
            <div className="kachel__bild">
              <Vorschaubild pfad={t.pfad} />
            </div>
            <div className="kachel__titel">{t.kurztitel || t.dateiname}</div>
            <div className="kachel__datum">
              {datumDe(t.aufnahmeDatum)}
              {t.seite ? ` · S. ${t.seite}` : ""}
            </div>
          </div>
        );
      }}
    />
  );
}
