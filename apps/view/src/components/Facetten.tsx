import { useEffect, useRef, useState } from "react";
import { zahlDe } from "../api";
import type { Facette } from "../types";

/** So viele Werte je Facette sind im Aufklapper sofort sichtbar; der Rest über „weitere“. */
export const SICHTBAR = 12;

interface Props {
  facetten: Facette[];
  /** Auswahl: Feld → gewählte Werte */
  auswahl: Record<string, string[]>;
  onChange: (auswahl: Record<string, string[]>) => void;
}

/**
 * Stufe 3, kompakt: eine Leiste mit einem Knopf je Facette (gewählte Werte als Zahl am Knopf und als
 * entfernbare Chips daneben). Ein Klick auf den Knopf klappt die Werte mit Anzahl darunter auf.
 */
export default function Facetten({ facetten, auswahl, onChange }: Props) {
  const [offen, setOffen] = useState<string | null>(null);
  const [alle, setAlle] = useState<Record<string, boolean>>({});
  const bereich = useRef<HTMLDivElement>(null);

  // Esc und Klick außerhalb schließen den Aufklapper
  useEffect(() => {
    if (!offen) return;
    const taste = (e: KeyboardEvent) => e.key === "Escape" && setOffen(null);
    const maus = (e: MouseEvent) => !bereich.current?.contains(e.target as Node) && setOffen(null);
    window.addEventListener("keydown", taste);
    window.addEventListener("mousedown", maus);
    return () => {
      window.removeEventListener("keydown", taste);
      window.removeEventListener("mousedown", maus);
    };
  }, [offen]);

  const umschalten = (feld: string, wert: string) => {
    const bisher = auswahl[feld] ?? [];
    const neu = bisher.includes(wert) ? bisher.filter((w) => w !== wert) : [...bisher, wert];
    onChange({ ...auswahl, [feld]: neu });
  };

  const sichtbar = facetten.filter((f) => f.werte.length > 0);
  if (sichtbar.length === 0) return null;
  const aktuell = sichtbar.find((f) => f.feld === offen);
  const titel = (feld: string) => sichtbar.find((f) => f.feld === feld)?.titel ?? feld;
  const gewaehlt = Object.entries(auswahl).flatMap(([feld, werte]) => werte.map((wert) => ({ feld, wert })));

  return (
    <div className="facetten" ref={bereich}>
      <span className="facetten__titel">Eigenschaften</span>
      <div className="facetten__leiste">
        {sichtbar.map((f) => {
          const n = auswahl[f.feld]?.length ?? 0;
          return (
            <button
              key={f.feld}
              className={`facette__knopf ${n ? "facette__knopf--aktiv" : ""} ${offen === f.feld ? "offen" : ""}`}
              aria-expanded={offen === f.feld}
              onClick={() => setOffen(offen === f.feld ? null : f.feld)}
            >
              {f.titel}
              {n > 0 && <span className="zaehler">{n}</span>}
              <svg viewBox="0 0 24 24" aria-hidden>
                <path d="M7 10l5 5 5-5" />
              </svg>
            </button>
          );
        })}
        {gewaehlt.map(({ feld, wert }) => (
          <button
            key={`${feld}:${wert}`}
            className="chip chip--aktiv chip--klein"
            aria-label={`${wert} entfernen`}
            title={titel(feld)}
            onClick={() => umschalten(feld, wert)}
          >
            {wert} <span aria-hidden>×</span>
          </button>
        ))}
      </div>

      {aktuell && (
        <div className="facette__werte chips">
          {(alle[aktuell.feld]
            ? aktuell.werte
            : aktuell.werte.filter((w, i) => i < SICHTBAR || (auswahl[aktuell.feld] ?? []).includes(w.wert))
          ).map((w) => {
            const aktiv = (auswahl[aktuell.feld] ?? []).includes(w.wert);
            return (
              <button
                key={w.wert}
                aria-label={`${w.wert} (${w.anzahl})`}
                className={`chip chip--klein ${aktiv ? "chip--aktiv" : ""} ${w.anzahl === 0 ? "chip--leer" : ""}`}
                onClick={() => umschalten(aktuell.feld, w.wert)}
              >
                {w.wert}
                <span className="chip__anzahl">{zahlDe(w.anzahl)}</span>
              </button>
            );
          })}
          {aktuell.werte.length > SICHTBAR && (
            <button
              className="chip chip--klein chip--mehr"
              onClick={() => setAlle({ ...alle, [aktuell.feld]: !alle[aktuell.feld] })}
            >
              {alle[aktuell.feld] ? "weniger" : `${aktuell.werte.length - SICHTBAR} weitere`}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
