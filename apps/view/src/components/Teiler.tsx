import { useRef } from "react";
import { TEILER } from "../teiler";

interface Props {
  /** Arbeitsbereich – rechte Kante ist der Bezug für die Breite */
  bereich: () => HTMLElement | null;
  /** gewünschte Breite des Detailbereichs während des Ziehens */
  onZiehen: (breite: number) => void;
  onFertig: () => void;
  /** Doppelklick: Standardbreite */
  onZuruecksetzen: () => void;
}

/** Schieber zwischen Ergebnis und Detailbereich (v0.0.6). */
export default function Teiler({ bereich, onZiehen, onFertig, onZuruecksetzen }: Props) {
  const zieht = useRef(false);
  const breite = (x: number) => {
    const r = bereich()?.getBoundingClientRect();
    return r ? r.right - x - TEILER / 2 : 0;
  };
  return (
    <div
      className="teiler"
      role="separator"
      aria-orientation="vertical"
      aria-label="Breite der Details"
      title="Ziehen ändert die Breite · Doppelklick: Standard"
      onPointerDown={(e) => {
        zieht.current = true;
        e.currentTarget.setPointerCapture?.(e.pointerId);
        document.body.classList.add("zieht");
        e.preventDefault();
      }}
      onPointerMove={(e) => zieht.current && onZiehen(breite(e.clientX))}
      onPointerUp={(e) => {
        if (!zieht.current) return;
        zieht.current = false;
        e.currentTarget.releasePointerCapture?.(e.pointerId);
        document.body.classList.remove("zieht");
        onFertig();
      }}
      onDoubleClick={onZuruecksetzen}
    />
  );
}
