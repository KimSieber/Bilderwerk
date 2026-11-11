import { useEffect, useRef, useState } from "react";
import { DATEITYPEN, type Facette, type SuchFilter } from "../types";
import Facetten from "./Facetten";

interface Props {
  filter: SuchFilter;
  onChange: (f: SuchFilter) => void;
  /** true, sobald Ergebnisse da sind → Suchfeld wandert nach oben */
  kompakt: boolean;
  /** Facetten mit Anzahl für die aktuellen Kriterien */
  facetten?: Facette[];
}

export default function SearchPanel({ filter, onChange, kompakt, facetten = [] }: Props) {
  const [mehr, setMehr] = useState(false);
  const eingabe = useRef<HTMLInputElement>(null);

  useEffect(() => eingabe.current?.focus(), []);

  // ⌘F fokussiert das Suchfeld
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.metaKey && e.key.toLowerCase() === "f") {
        e.preventDefault();
        eingabe.current?.focus();
        eingabe.current?.select();
      }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  const setze = <K extends keyof SuchFilter>(k: K, v: SuchFilter[K]) =>
    onChange({ ...filter, [k]: v });

  const typAktiv = (endungen: string[]) => endungen.every((e) => filter.endungen.includes(e));
  const typUmschalten = (endungen: string[]) =>
    setze(
      "endungen",
      typAktiv(endungen)
        ? filter.endungen.filter((e) => !endungen.includes(e))
        : [...filter.endungen, ...endungen],
    );

  const gewaehlteFacetten = Object.values(filter.facetten).reduce((n, w) => n + w.length, 0);

  const anzahlKriterien =
    [filter.name, filter.ordner, filter.datumVon, filter.datumBis].filter((s) => s.trim()).length +
    (filter.endungen.length ? 1 : 0) +
    (filter.mitDuplikaten ? 1 : 0) +
    gewaehlteFacetten;

  const hatEtwas =
    !!filter.freitext.trim() ||
    [filter.name, filter.ordner, filter.datumVon, filter.datumBis].some((s) => s.trim()) ||
    filter.endungen.length > 0 ||
    gewaehlteFacetten > 0;

  /** Zurück zum Start: alle Kriterien leeren, Kriterien zuklappen. */
  const alles = () => {
    onChange({
      ...filter,
      freitext: "",
      name: "",
      ordner: "",
      datumVon: "",
      datumBis: "",
      endungen: [],
      facetten: {},
    });
    setMehr(false);
    eingabe.current?.focus();
  };

  return (
    <section className={`suche ${kompakt ? "suche--kompakt" : ""}`}>
      <div className="suchfeld">
        <svg className="suchfeld__lupe" viewBox="0 0 24 24" aria-hidden>
          <circle cx="11" cy="11" r="7" />
          <path d="M20 20l-3.5-3.5" />
        </svg>
        <input
          ref={eingabe}
          type="text"
          placeholder="Bilder suchen …"
          value={filter.freitext}
          spellCheck={false}
          autoCorrect="off"
          onChange={(e) => setze("freitext", e.target.value)}
          onKeyDown={(e) => e.key === "Escape" && alles()}
        />
        {hatEtwas && (
          <button className="suchfeld__leeren" onClick={alles} title="Suche leeren (Esc)">
            <svg viewBox="0 0 24 24" aria-hidden>
              <path d="M7 7l10 10M17 7L7 17" />
            </svg>
          </button>
        )}
        <button
          className={`suchfeld__mehr ${mehr ? "aktiv" : ""}`}
          onClick={() => setMehr(!mehr)}
          title="Weitere Kriterien"
        >
          Kriterien{anzahlKriterien > 0 && <span className="zaehler">{anzahlKriterien}</span>}
        </button>
      </div>

      <div className={`kriterien ${mehr ? "kriterien--offen" : ""}`}>
        <div className="kriterien__innen">
          <label className="feld">
            <span>Dateiname</span>
            <input value={filter.name} onChange={(e) => setze("name", e.target.value)} spellCheck={false} />
          </label>
          <label className="feld">
            <span>Ordner</span>
            <input value={filter.ordner} onChange={(e) => setze("ordner", e.target.value)} spellCheck={false} />
          </label>
          <label className="feld feld--datum">
            <span>Datum von</span>
            <input
              value={filter.datumVon}
              placeholder="JJJJ[-MM[-TT]]"
              onChange={(e) => setze("datumVon", e.target.value)}
            />
          </label>
          <label className="feld feld--datum">
            <span>bis</span>
            <input
              value={filter.datumBis}
              placeholder="JJJJ[-MM[-TT]]"
              onChange={(e) => setze("datumBis", e.target.value)}
            />
          </label>
          <div className="feld feld--typen">
            <span>Dateityp</span>
            <div className="chips">
              {DATEITYPEN.map((t) => (
                <button
                  key={t.label}
                  className={`chip ${typAktiv(t.endungen) ? "chip--aktiv" : ""}`}
                  onClick={() => typUmschalten(t.endungen)}
                >
                  {t.label}
                </button>
              ))}
            </div>
          </div>
          <label className="schalter">
            <input
              type="checkbox"
              checked={filter.mitDuplikaten}
              onChange={(e) => setze("mitDuplikaten", e.target.checked)}
            />
            <span>Duplikate einbeziehen</span>
          </label>
          <Facetten facetten={facetten} auswahl={filter.facetten} onChange={(f) => setze("facetten", f)} />
        </div>
      </div>
    </section>
  );
}
