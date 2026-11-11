// Breite des Detailbereichs (v0.0.6): per Schieber änderbar, gemerkt, immer in sinnvollen Grenzen.

export const DETAIL_MIN = 280;
export const DETAIL_STANDARD = 380;
/** Mindestbreite der Liste, damit Titel/Ablage/Datum lesbar bleiben */
export const LISTE_MIN = 420;
/** Kopfzeile (Kachelregler, Raster/Liste, Details) muss auch bei kleinster Kachel ganz passen */
export const KOPF_MIN = 250;
/** Breite des Schiebers zwischen Ergebnis und Details */
export const TEILER = 14;

/** Platz, den das Ergebnis mindestens braucht: eine Kachel in der aktuellen Größe bzw. die Liste */
export function ergebnisMin(ansicht: "raster" | "liste", kachel: number): number {
  return ansicht === "liste" ? LISTE_MIN : Math.max(KOPF_MIN, kachel + 2 * 6 + 16); // Rand des Rasters + Bildlaufleiste
}

/** Gewünschte Breite auf [DETAIL_MIN, gesamt − Teiler − Ergebnis-Minimum] begrenzen */
export function begrenzen(wunsch: number, gesamt: number, ergebnis: number): number {
  const max = Math.max(DETAIL_MIN, gesamt - TEILER - ergebnis);
  return Math.round(Math.min(max, Math.max(DETAIL_MIN, wunsch)));
}
