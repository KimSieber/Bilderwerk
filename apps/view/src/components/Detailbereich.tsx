import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, zahlDe } from "../api";
import { aufbereiten } from "../detailfelder";
import type { Details, Treffer } from "../types";
import Vorschaubild from "./Vorschaubild";

interface Props {
  /** ausgewählter Treffer (null = keine Auswahl) */
  treffer: Treffer | null;
  /** Position in der Trefferliste (1-basiert) und Anzahl */
  position: [number, number] | null;
  /** Breite in px (Schieber); fehlt → aus dem CSS */
  breite?: number;
  /** Großansicht öffnen (Knopf, Doppelklick aufs Bild) */
  onGross?: () => void;
  /** zählt hoch, wenn die DB neu geladen wurde → Angaben neu holen */
  stand?: number;
}

type Meldung = { text: string; fehler: boolean } | null;

/**
 * Detailbereich rechts (v0.0.6): 640-px-Vorschau (DB/Cache, kein Nachladen) und alle Angaben aus der DB.
 * „Großansicht“ (Knopf, Doppelklick, Leertaste) öffnet das Bild über dem ganzen Fenster (Grossansicht.tsx).
 */
export default function Detailbereich({ treffer, position, breite, onGross, stand = 0 }: Props) {
  // Duplikat angeklickt → dieses anzeigen, bis sich die Auswahl ändert
  const [anderes, setAnderes] = useState<{ id: number; pfad: string } | null>(null);
  const id = anderes?.id ?? treffer?.id ?? null;
  const pfad = anderes?.pfad ?? treffer?.pfad ?? null;

  const [daten, setDaten] = useState<Details | null>(null);
  const [fehler, setFehler] = useState<string | null>(null);
  const [meldung, setMeldung] = useState<Meldung>(null);
  const [beschaeftigt, setBeschaeftigt] = useState<string | null>(null);
  const anfrage = useRef(0);

  useEffect(() => setAnderes(null), [treffer?.id]);

  // Angaben laden – nur die jüngste Antwort zählt (schnelles Blättern)
  useEffect(() => {
    setMeldung(null);
    if (id === null) return;
    const nr = ++anfrage.current;
    setFehler(null);
    api
      .details(id)
      .then((d) => nr === anfrage.current && setDaten(d))
      .catch((m) => nr === anfrage.current && setFehler(String(m)));
  }, [id, stand]);

  // Tastenkürzel: ⌘O Öffnen · ⌥⌘R Im Finder · ⇧⌘S Sichern unter (v0.0.7)
  const kuerzel = useRef<(e: KeyboardEvent) => void>(() => {});
  useEffect(() => {
    const h = (e: KeyboardEvent) => kuerzel.current(e);
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  if (id === null || !pfad) {
    kuerzel.current = () => {};
    return (
      <aside className="detail detail--leer">
        <p>Bild auswählen – Klick oder Pfeiltasten.</p>
      </aside>
    );
  }

  const aktion = async (name: string, f: () => Promise<string | void>) => {
    setMeldung(null);
    setBeschaeftigt(name);
    try {
      const text = await f();
      if (text) setMeldung({ text, fehler: false });
    } catch (m) {
      setMeldung({ text: String(m), fehler: true });
    } finally {
      setBeschaeftigt(null);
    }
  };

  kuerzel.current = (e: KeyboardEvent) => {
    if (!e.metaKey || e.ctrlKey || beschaeftigt) return;
    const k = e.key.toLowerCase();
    if (k === "o" && !e.altKey && !e.shiftKey) {
      e.preventDefault();
      aktion("oeffnen", () => api.originalOeffnen(pfad));
    } else if ((k === "r" || e.code === "KeyR") && e.altKey && !e.shiftKey) {
      e.preventDefault();
      aktion("finder", () => api.imFinderZeigen(pfad));
    } else if (k === "s" && e.shiftKey && !e.altKey) {
      e.preventDefault();
      aktion("sichern", sichern);
    }
  };

  const sichern = async () => {
    const ziel = await save({ title: "Original sichern unter", defaultPath: await api.dateinameVorschlag(pfad) });
    if (!ziel) return;
    await api.sichernUnter(pfad, ziel);
    return `Gesichert: ${ziel}`;
  };

  const a = daten && daten.id === id ? aufbereiten(daten) : null;
  const titel = a?.titel ?? (anderes ? "" : treffer?.kurztitel || treffer?.dateiname || "");

  return (
    <aside className="detail" aria-label="Details" style={breite ? { flexBasis: breite } : undefined}>
      <div className="detail__bild" onDoubleClick={onGross} title="Doppelklick: Großansicht">
        <Vorschaubild pfad={pfad} />
      </div>

      <div className="detail__knoepfe">
        <button className="knopf knopf--klein" onClick={onGross} title="Großansicht (Leertaste)">
          Großansicht
        </button>
        <button className="knopf knopf--klein" disabled={!!beschaeftigt} onClick={() => aktion("oeffnen", () => api.originalOeffnen(pfad))} title="In der Standard-App öffnen (⌘O)">
          Öffnen
        </button>
        <button className="knopf knopf--klein" disabled={!!beschaeftigt} onClick={() => aktion("finder", () => api.imFinderZeigen(pfad))} title="Im Finder zeigen (⌥⌘R)">
          Im Finder
        </button>
        <button className="knopf knopf--klein" disabled={!!beschaeftigt} onClick={() => aktion("sichern", sichern)} title="Original sichern unter (⇧⌘S)">
          Sichern unter …
        </button>
        <button
          className="knopf knopf--klein"
          disabled={!!beschaeftigt}
          title="Teilen"
          aria-label="Teilen"
          onClick={(e) => {
            const r = e.currentTarget.getBoundingClientRect();
            aktion("teilen", () => api.teilen(pfad, [r.left, r.top, r.width, r.height]));
          }}
        >
          <svg viewBox="0 0 24 24" aria-hidden>
            <path d="M12 3v12M8 7l4-4 4 4M5 12v7a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2v-7" />
          </svg>
        </button>
      </div>
      {beschaeftigt && <div className="detail__meldung">Original wird bereitgestellt …</div>}
      {meldung && <div className={`detail__meldung ${meldung.fehler ? "detail__meldung--fehler" : ""}`}>{meldung.text}</div>}

      {anderes && (
        <button className="detail__zurueck" onClick={() => setAnderes(null)}>
          ← zurück zur Auswahl
        </button>
      )}
      <h2 className="detail__titel">{titel}</h2>
      {a?.beschreibung && <p className="detail__beschreibung">{a.beschreibung}</p>}
      {fehler && <div className="detail__meldung detail__meldung--fehler">{fehler}</div>}

      {a && (
        <>
          {a.gruppen.slice(0, 2).map((g) => <Abschnitt key={g.titel} g={g} />)}
          {a.chips.map((c) => (
            <section key={c.titel} className="detail__gruppe">
              <h3>{c.titel}</h3>
              <div className="detail__chips">
                {c.werte.map((w) => (
                  <span key={w} className="detail__chip">{w}</span>
                ))}
              </div>
            </section>
          ))}
          {a.gruppen.slice(2).map((g) => <Abschnitt key={g.titel} g={g} />)}
          {daten!.duplikate.length > 0 && (
            <section className="detail__gruppe">
              <h3>{daten!.duplikate.length === 1 ? "1 weiteres Exemplar" : `${daten!.duplikate.length} weitere Exemplare`}</h3>
              {daten!.duplikate.map((v) => (
                <button key={v.id} className="detail__verwandt" title={v.pfad} onClick={() => setAnderes({ id: v.id, pfad: v.pfad })}>
                  {v.pfad}
                  {v.status === "duplikat" ? "" : " (Original)"}
                </button>
              ))}
            </section>
          )}
        </>
      )}

      {position && !anderes && (
        <div className="detail__fuss">
          <span>← → blättern · Leertaste Großansicht</span>
          <span>
            {zahlDe(position[0])} / {zahlDe(position[1])}
          </span>
        </div>
      )}
    </aside>
  );
}

function Abschnitt({ g }: { g: ReturnType<typeof aufbereiten>["gruppen"][number] }) {
  const inhalt = g.block ? (
    <p className="detail__text">{g.zeilen[0].wert}</p>
  ) : (
    <dl className="detail__liste">
      {g.zeilen.map((z) => (
        <div key={z.label} className="detail__zeile">
          <dt>{z.label}</dt>
          <dd>{z.wert}</dd>
        </div>
      ))}
    </dl>
  );
  if (g.zu)
    return (
      <details className="detail__gruppe detail__gruppe--zu">
        <summary>{g.titel}</summary>
        {inhalt}
      </details>
    );
  return (
    <section className="detail__gruppe">
      <h3>{g.titel}</h3>
      {inhalt}
    </section>
  );
}
