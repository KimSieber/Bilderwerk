import { useEffect, useRef, useState } from "react";
import { api, groesseDe, zahlDe } from "../api";
import type { CacheInfo, DbInfo, DropboxStatus } from "../types";
import { alleNeuLaden, nurOnlineNeuLaden } from "../vorschau";

interface Props {
  info: DbInfo;
  onWechseln: () => void;
}

/** Kleines Menü oben rechts: Stand der Datenbank-Sicherung, Wechsel der Datei, Dropbox-Anmeldung. */
export default function DbMenu({ info, onWechseln }: Props) {
  const [offen, setOffen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const [dropbox, setDropbox] = useState<DropboxStatus | null>(null);
  const [dbxWartet, setDbxWartet] = useState(false);
  const [dbxFehler, setDbxFehler] = useState<string | null>(null);

  const [cache, setCache] = useState<CacheInfo | null>(null);
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
  const [leert, setLeert] = useState(false);

  // Status bei jedem Öffnen frisch abfragen; ohne Tauri/App-Schlüssel bleibt der Bereich weg
  useEffect(() => {
    if (!offen) return;
    api
      .dropboxStatus()
      .then(setDropbox)
      .catch(() => setDropbox(null));
    api
      .cacheInfo()
      .then(setCache)
      .catch(() => setCache(null));
  }, [offen]);

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
          {cache && (
            <div className="dbmenu__cache">
              <div className="dbmenu__zeile">
                <span className="leise">Vorschau-Cache</span>
                <span>{groesseDe(cache.bytes)}</span>
              </div>
              <button className="knopf" onClick={cacheLeeren} disabled={leert || cache.dateien === 0}>
                {leert ? "Wird geleert …" : "Cache leeren"}
              </button>
            </div>
          )}
          {dropbox?.eingerichtet && (
            <div className="dbmenu__dropbox">
              <div className="dbmenu__zeile">
                <span className="leise">Dropbox</span>
                <span>{dropbox.angemeldet ? "angemeldet" : "nicht angemeldet"}</span>
              </div>
              {dbxFehler && <div className="dbmenu__fehler">{dbxFehler}</div>}
              {dropbox.angemeldet ? (
                <button className="knopf" onClick={abmelden}>
                  Von Dropbox abmelden
                </button>
              ) : (
                <button className="knopf" onClick={anmelden} disabled={dbxWartet}>
                  {dbxWartet ? "Anmeldung im Browser …" : "Bei Dropbox anmelden…"}
                </button>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
