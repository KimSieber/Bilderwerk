import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import symbol from "../../src-tauri/icons/128x128@2x.png";

export const WEBSEITE = "https://bilderwerk.sieber-bw.de";
export const EMAIL = "Bilderwerk@sieber-bw.de";
export const LIZENZ_URL = "https://polyformproject.org/licenses/noncommercial/1.0.0";

/** Fenster „Über Bilderwerk View“ (v0.3) – Version aus tauri.conf.json, klickbare Webseite und E-Mail. */
export default function Ueber() {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion().then(setVersion, () => setVersion(null));
  }, []);

  const oeffnen = (url: string) => (e: React.MouseEvent) => {
    e.preventDefault();
    openUrl(url).catch(() => {});
  };

  return (
    <div className="ueber">
      <img className="ueber__symbol" src={symbol} alt="" />
      <h1>Bilderwerk View</h1>
      <div className="ueber__version">{version ? `Version ${version}` : " "}</div>
      <p className="ueber__text">Suche im KI-Bildkatalog von Bilderwerk.</p>
      <p>
        <a href={WEBSEITE} onClick={oeffnen(WEBSEITE)}>
          bilderwerk.sieber-bw.de
        </a>
        <br />
        <a href={`mailto:${EMAIL}`} onClick={oeffnen(`mailto:${EMAIL}`)}>
          {EMAIL}
        </a>
      </p>
      <p className="ueber__klein">
        © 2026 Kim Sieber
        <br />
        Lizenz:{" "}
        <a href={LIZENZ_URL} onClick={oeffnen(LIZENZ_URL)}>
          PolyForm Noncommercial 1.0.0
        </a>
        <br />
        ausgenommen Icons und App-Symbole
      </p>
    </div>
  );
}
