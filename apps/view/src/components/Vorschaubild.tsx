import { useState } from "react";
import { useVorschau } from "../vorschau";

interface Props {
  pfad: string;
}

/** Vorschau mit weichem Einblenden. Während des Ladens bleibt die Fläche ruhig (kein Spinner). */
export default function Vorschaubild({ pfad }: Props) {
  const { status } = useVorschau(pfad);
  const [geladen, setGeladen] = useState(false);

  if (status.art === "bild") {
    return (
      <img
        className={`vorschau__bild ${geladen ? "geladen" : ""}`}
        src={status.url}
        alt=""
        draggable={false}
        decoding="async"
        onLoad={() => setGeladen(true)}
      />
    );
  }

  if (status.art === "nurOnline") {
    return (
      <div className="vorschau__symbol" title="Nur online (Dropbox) – bitte bei Dropbox anmelden">
        <svg viewBox="0 0 24 24" aria-hidden>
          <path d="M7 18h10a4 4 0 0 0 .6-7.95A6 6 0 0 0 6.2 9.1 4.5 4.5 0 0 0 7 18z" />
        </svg>
      </div>
    );
  }

  if (status.art === "fehlt" || status.art === "fehler") {
    return (
      <div
        className="vorschau__symbol"
        title={status.art === "fehlt" ? "Datei nicht gefunden" : status.meldung}
      >
        <svg viewBox="0 0 24 24" aria-hidden>
          <rect x="4" y="5" width="16" height="14" rx="2" />
          <path d="M4 5l16 14" />
        </svg>
      </div>
    );
  }

  return null; // lädt
}
