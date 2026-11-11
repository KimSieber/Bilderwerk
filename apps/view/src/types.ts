// Spiegel der Rust-Strukturen (serde camelCase)

export interface SuchFilter {
  freitext: string;
  name: string;
  ordner: string;
  datumVon: string;
  datumBis: string;
  endungen: string[];
  mitDuplikaten: boolean;
  limit?: number;
  /** Facetten-Auswahl (Stufe 3): Feld → Werte; innerhalb ODER, zwischen Feldern UND */
  facetten: Record<string, string[]>;
}

export interface FacettenWert {
  wert: string;
  anzahl: number;
}

/** Eine Facette mit Anzahl je Wert (Befehl `facetten`) */
export interface Facette {
  /** Schlüssel in SuchFilter.facetten, z. B. "motiv" */
  feld: string;
  titel: string;
  werte: FacettenWert[];
}

export interface Treffer {
  id: number;
  pfad: string;
  relativerPfad: string | null;
  archivWurzel: string | null;
  ordner: string | null;
  dateiname: string | null;
  endung: string | null;
  kurztitel: string | null;
  aufnahmeDatum: string | null;
  seite: number | null;
  breite: number | null;
  hoehe: number | null;
  status: string | null;
}

export interface Ergebnis {
  gesamt: number;
  treffer: Treffer[];
  dauerMs: number;
}

export interface DbInfo {
  pfad: string;
  schemaVersion: number;
  bilder: number;
  duplikate: number;
  geaendert: number | null;
  groesse: number;
  /** Vorschauen in der Katalog-DB (Tabelle `vorschauen`); null = ältere DB ohne Vorschauen */
  vorschauen: number | null;
}

export interface CacheInfo {
  bytes: number;
  dateien: number;
}

export interface DropboxStatus {
  /** App-Schlüssel in view.local.json vorhanden */
  eingerichtet: boolean;
  /** Anmeldung im Schlüsselbund */
  angemeldet: boolean;
}

export const LEERER_FILTER: SuchFilter = {
  freitext: "",
  name: "",
  ordner: "",
  datumVon: "",
  datumBis: "",
  endungen: [],
  mitDuplikaten: false,
  facetten: {},
};

/** Dateitypen als Gruppen – eine Auswahl steht für mehrere Endungen. */
export const DATEITYPEN: { label: string; endungen: string[] }[] = [
  { label: "JPEG", endungen: [".jpg", ".jpeg"] },
  { label: "HEIC", endungen: [".heic", ".heif"] },
  { label: "PNG", endungen: [".png"] },
  { label: "WebP", endungen: [".webp"] },
  { label: "TIFF", endungen: [".tif", ".tiff"] },
  { label: "PDF", endungen: [".pdf"] },
];

/** Eine Spalte aus `bilder` mit Wert (Befehl `details`, v0.0.6) */
export interface DetailFeld {
  name: string;
  wert: string | number;
}

export interface Verwandt {
  id: number;
  pfad: string;
  status: string | null;
}

/** Alle Angaben eines Bildes für den Detailbereich */
export interface Details {
  id: number;
  pfad: string;
  /** Spalten mit Wert in Tabellenreihenfolge */
  felder: DetailFeld[];
  /** Einzelbegriffe je Typ (schlagwort, merkmal, stimmung, stichwort) */
  begriffe: Record<string, string[]>;
  /** Original und weitere Duplikate (ohne das Bild selbst) */
  duplikate: Verwandt[];
}
