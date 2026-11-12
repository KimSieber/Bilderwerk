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
  /** App-Schlüssel eingetragen (eigener oder mitgelieferter) */
  eingerichtet: boolean;
  /** Anmeldung im Schlüsselbund */
  angemeldet: boolean;
}

/** Speicheranbieter je Ordner (v0.3) – es gilt der längste passende Ordner */
export type AnbieterArt = "lokal" | "file-provider" | "dropbox-klassisch";

export interface Regel {
  pfad: string;
  art: AnbieterArt;
}

export const ANBIETER_ARTEN: { art: AnbieterArt; label: string; hilfe: string }[] = [
  { art: "lokal", label: "Lokal", hilfe: "Dateien auf diesem Rechner; „nur online“ bleibt unangetastet" },
  { art: "file-provider", label: "Cloud (File Provider)", hilfe: "iCloud, OneDrive, Google Drive, neue Dropbox – lädt beim Lesen nach" },
  { art: "dropbox-klassisch", label: "Dropbox (klassisch)", hilfe: "Ordner ~/Dropbox – „nur online“ über die Dropbox-API" },
];

/** Einstellungen für den Dialog (Befehl `einstellungen`, v0.3) */
export interface Einstellungen {
  dbPfad: string | null;
  cacheGrenzeMb: number;
  anbieter: Regel[];
  /** wirksamer App-Schlüssel, "" = keine Dropbox-API */
  dropboxAppKey: string;
  /** mitgelieferter Schlüssel */
  standardAppKey: string;
}

/** Änderung aus dem Dialog (Befehl `einstellungen_setzen`) */
export interface EinstellungenAenderung {
  cacheGrenzeMb: number;
  anbieter: Regel[];
  dropboxAppKey: string;
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
  // RAW (seit Work v0.2, gleiche Liste wie shared/bilderwerk/bild.py RAW_ENDUNGEN)
  {
    label: "RAW",
    endungen: [".cr2", ".cr3", ".nef", ".nrw", ".arw", ".dng", ".orf", ".raf", ".rw2", ".pef", ".srw"],
  },
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
