import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, groesseDe } from "../api";
import { ANBIETER_ARTEN } from "../types";
import type { AnbieterArt, CacheInfo, DbInfo, DropboxStatus, Einstellungen, Regel } from "../types";
import { alleNeuLaden, nurOnlineNeuLaden } from "../vorschau";

interface Props {
  /** geöffnete DB (null = noch keine) */
  db: DbInfo | null;
  onDbWechseln: () => void;
  onSchliessen: () => void;
}

const gleich = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** Einstellungsdialog (v0.3): Datenbank, Vorschau-Cache, Speicheranbieter je Ordner, Dropbox-App-Schlüssel.
 *  Änderungen wirken nach „Übernehmen“ sofort – ohne Neustart. */
export default function EinstellungenDialog({ db, onDbWechseln, onSchliessen }: Props) {
  const [stand, setStand] = useState<Einstellungen | null>(null);
  const [ladeFehler, setLadeFehler] = useState<string | null>(null);
  // Entwurf
  const [grenze, setGrenze] = useState("");
  const [regeln, setRegeln] = useState<Regel[]>([]);
  const [key, setKey] = useState("");

  const [speichert, setSpeichert] = useState(false);
  const [fehler, setFehler] = useState<string | null>(null);
  const [gespeichert, setGespeichert] = useState(false);

  const [cache, setCache] = useState<CacheInfo | null>(null);
  const [leert, setLeert] = useState(false);
  const [dropbox, setDropbox] = useState<DropboxStatus | null>(null);
  const [dbxWartet, setDbxWartet] = useState(false);
  const [dbxFehler, setDbxFehler] = useState<string | null>(null);

  const uebernehmenIns = (e: Einstellungen) => {
    setStand(e);
    setGrenze(String(e.cacheGrenzeMb));
    setRegeln(e.anbieter);
    setKey(e.dropboxAppKey);
  };

  useEffect(() => {
    api.einstellungen().then(uebernehmenIns, (e) => setLadeFehler(String(e)));
    api.cacheInfo().then(setCache, () => setCache(null));
    api.dropboxStatus().then(setDropbox, () => setDropbox(null));
  }, []);

  // Tasten gehören dem Dialog: Raster, Suchfeld, Großansicht merken nichts (Tippen bleibt möglich); Esc schließt
  const schliessen = useRef(onSchliessen);
  schliessen.current = onSchliessen;
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        schliessen.current();
      }
      e.stopImmediatePropagation();
    };
    window.addEventListener("keydown", h, true);
    return () => window.removeEventListener("keydown", h, true);
  }, []);

  const geaendert =
    stand !== null &&
    (String(stand.cacheGrenzeMb) !== grenze.trim() || !gleich(stand.anbieter, regeln) || stand.dropboxAppKey !== key.trim());
  const keyGeaendert = stand !== null && stand.dropboxAppKey !== key.trim();
  const dropboxRegeln = regeln.some((r) => r.art === "dropbox-klassisch");

  const regelAendern = (i: number, r: Partial<Regel>) => setRegeln(regeln.map((x, j) => (j === i ? { ...x, ...r } : x)));
  const regelEntfernen = (i: number) => setRegeln(regeln.filter((_, j) => j !== i));
  const ordnerWaehlen = async (i: number) => {
    const pfad = await open({ title: "Ordner für die Regel wählen", directory: true, multiple: false });
    if (typeof pfad === "string") regelAendern(i, { pfad });
  };
  const vorschlagen = async () => {
    try {
      const v = await api.anbieterVorschlaege();
      const neu = v.filter((x) => !regeln.some((r) => r.pfad === x.pfad));
      setRegeln([...regeln, ...neu]);
    } catch (e) {
      setFehler(String(e));
    }
  };

  const uebernehmen = async () => {
    const mb = Number(grenze.trim());
    if (!Number.isInteger(mb) || mb <= 0) {
      setFehler("Die Cache-Grenze bitte als ganze Zahl in MB angeben.");
      return;
    }
    setSpeichert(true);
    setFehler(null);
    try {
      const e = await api.einstellungenSetzen({ cacheGrenzeMb: mb, anbieter: regeln, dropboxAppKey: key.trim() });
      uebernehmenIns(e);
      setGespeichert(true);
      window.setTimeout(() => setGespeichert(false), 2500);
      api.dropboxStatus().then(setDropbox, () => setDropbox(null));
      api.cacheInfo().then(setCache, () => {});
    } catch (e) {
      setFehler(String(e));
    } finally {
      setSpeichert(false);
    }
  };

  const cacheLeeren = async () => {
    setLeert(true);
    try {
      setCache(await api.cacheLeeren());
      alleNeuLaden();
    } catch {
      /* Anzeige bleibt */
    } finally {
      setLeert(false);
    }
  };

  const anmelden = async () => {
    setDbxWartet(true);
    setDbxFehler(null);
    try {
      const s = await api.dropboxAnmelden();
      setDropbox(s);
      if (s.angemeldet) nurOnlineNeuLaden();
    } catch (e) {
      setDbxFehler(String(e));
    } finally {
      setDbxWartet(false);
    }
  };

  const abmelden = async () => {
    setDbxFehler(null);
    try {
      setDropbox(await api.dropboxAbmelden());
    } catch (e) {
      setDbxFehler(String(e));
    }
  };

  const dbPfad = db?.pfad ?? stand?.dbPfad ?? null;

  return (
    <div className="dialog" role="dialog" aria-modal="true" aria-label="Einstellungen" onMouseDown={(e) => e.target === e.currentTarget && onSchliessen()}>
      <div className="dialog__fenster">
        <div className="dialog__kopf">
          <h2>Einstellungen</h2>
          <button className="dialog__zu" onClick={onSchliessen} title="Schließen (Esc)" aria-label="Schließen">
            <svg viewBox="0 0 16 16" aria-hidden>
              <path d="M4 4l8 8M12 4l-8 8" />
            </svg>
          </button>
        </div>

        {ladeFehler && <div className="dialog__fehler">{ladeFehler}</div>}

        <section className="dialog__abschnitt">
          <h3>Datenbank</h3>
          <div className="dialog__pfad" title={dbPfad ?? undefined}>
            {dbPfad ?? "Noch keine Datenbank ausgewählt."}
          </div>
          <button className="knopf knopf--klein" onClick={onDbWechseln}>
            {dbPfad ? "Andere Datenbank…" : "Datenbank auswählen…"}
          </button>
        </section>

        <section className="dialog__abschnitt">
          <h3>Vorschau-Cache</h3>
          <div className="dialog__reihe">
            <label htmlFor="cache-grenze">Höchstens</label>
            <input
              id="cache-grenze"
              className="dialog__zahl"
              inputMode="numeric"
              value={grenze}
              onChange={(e) => setGrenze(e.target.value)}
            />
            <span className="leise">MB</span>
            <span className="dialog__fueller" />
            {cache && <span className="leise">belegt {groesseDe(cache.bytes)}</span>}
            {cache && (
              <button className="knopf knopf--klein" onClick={cacheLeeren} disabled={leert || cache.dateien === 0}>
                {leert ? "Wird geleert …" : "Cache leeren"}
              </button>
            )}
          </div>
        </section>

        <section className="dialog__abschnitt">
          <h3>Speicherorte</h3>
          <p className="dialog__hilfe">
            Wie View an Vorschau und Original eines Bildes kommt – je Ordner. Es gilt der längste passende Ordner, die
            Reihenfolge spielt keine Rolle. Ohne passende Regel: Lokal.
          </p>
          <div className="regeln">
            {regeln.map((r, i) => (
              <div className="regel" key={i}>
                <input
                  className="regel__pfad"
                  aria-label={`Ordner ${i + 1}`}
                  value={r.pfad}
                  placeholder="/Pfad/zum/Ordner"
                  onChange={(e) => regelAendern(i, { pfad: e.target.value })}
                />
                <button className="knopf knopf--klein" onClick={() => ordnerWaehlen(i)} title="Ordner wählen">
                  …
                </button>
                <select
                  aria-label={`Art ${i + 1}`}
                  value={r.art}
                  onChange={(e) => regelAendern(i, { art: e.target.value as AnbieterArt })}
                  title={ANBIETER_ARTEN.find((a) => a.art === r.art)?.hilfe}
                >
                  {ANBIETER_ARTEN.map((a) => (
                    <option key={a.art} value={a.art}>
                      {a.label}
                    </option>
                  ))}
                </select>
                <button className="regel__weg" onClick={() => regelEntfernen(i)} title="Regel entfernen" aria-label={`Regel ${i + 1} entfernen`}>
                  ×
                </button>
              </div>
            ))}
          </div>
          <div className="dialog__reihe">
            <button className="knopf knopf--klein" onClick={() => setRegeln([...regeln, { pfad: "", art: "lokal" }])}>
              Regel hinzufügen
            </button>
            <button className="knopf knopf--klein" onClick={vorschlagen} title="~/Dropbox, ~/Library/CloudStorage, iCloud">
              Erkannte Ordner vorschlagen
            </button>
          </div>
        </section>

        <section className="dialog__abschnitt">
          <h3>Dropbox</h3>
          <div className="dialog__reihe">
            <label htmlFor="dropbox-key">App-Schlüssel</label>
            <input
              id="dropbox-key"
              className="dialog__key"
              value={key}
              placeholder="leer = ohne Dropbox-API"
              spellCheck={false}
              onChange={(e) => setKey(e.target.value)}
            />
            {stand && key.trim() !== stand.standardAppKey && (
              <button className="knopf knopf--klein" onClick={() => setKey(stand.standardAppKey)} title="Mitgelieferte Dropbox-App „Bilderwerk View“">
                Standard
              </button>
            )}
          </div>
          {!key.trim() && dropboxRegeln && (
            <p className="dialog__hinweis">
              Ohne App-Schlüssel werden Dropbox-Ordner wie lokale Ordner behandelt: Dateien „nur online“ zeigen keine
              Vorschau und lassen sich nicht öffnen.
            </p>
          )}
          {dropbox?.eingerichtet && !keyGeaendert && (
            <div className="dialog__reihe">
              <span>{dropbox.angemeldet ? "angemeldet" : "nicht angemeldet"}</span>
              <span className="dialog__fueller" />
              {dropbox.angemeldet ? (
                <button className="knopf knopf--klein" onClick={abmelden}>
                  Von Dropbox abmelden
                </button>
              ) : (
                <button className="knopf knopf--klein" onClick={anmelden} disabled={dbxWartet}>
                  {dbxWartet ? "Anmeldung im Browser …" : "Bei Dropbox anmelden…"}
                </button>
              )}
            </div>
          )}
          {keyGeaendert && key.trim() && <p className="dialog__hilfe">Anmelden nach „Übernehmen“.</p>}
          {dbxFehler && <div className="dialog__fehler">{dbxFehler}</div>}
        </section>

        <div className="dialog__fuss">
          {fehler && <div className="dialog__fehler">{fehler}</div>}
          {gespeichert && !fehler && <span className="leise">Übernommen</span>}
          <span className="dialog__fueller" />
          <button className="knopf" onClick={onSchliessen}>
            {geaendert ? "Abbrechen" : "Schließen"}
          </button>
          <button className="knopf knopf--primaer" onClick={uebernehmen} disabled={!geaendert || speichert}>
            {speichert ? "Wird übernommen …" : "Übernehmen"}
          </button>
        </div>
      </div>
    </div>
  );
}
