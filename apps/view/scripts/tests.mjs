// npm test: alle Schichten nacheinander, Ausgabe wie gewohnt – am Ende eine Gesamtsumme.
// Rust (cargo test) → Frontend (Vitest) → Typprüfung (tsc). Exit-Code ≠ 0, sobald etwas scheitert.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";

/** Steuerzeichen für Farben entfernen (ESC[…m, ESC[…K, ESC(B von terminfo). */
export const ohneFarbe = (s) => s.replace(/\x1b(\[[0-9;?]*[A-Za-z]|\([A-Z0-9])/g, "");

function lauf(befehl, args) {
  return new Promise((fertig) => {
    let text = "";
    // Ausgabe wird mitgelesen (für die Summe) – daher Farben ausdrücklich einschalten,
    // sonst schalten cargo, die Rust-Tests und Vitest sie ab, weil sie kein Terminal sehen.
    const env = { ...process.env, FORCE_COLOR: "1", CARGO_TERM_COLOR: "always" };
    const p = spawn(befehl, args, { shell: process.platform === "win32", env });
    p.stdout.on("data", (d) => {
      process.stdout.write(d);
      text += d;
    });
    p.stderr.on("data", (d) => {
      process.stderr.write(d);
      text += d;
    });
    p.on("close", (code) => fertig({ code: code ?? 1, text: ohneFarbe(text) }));
    p.on("error", (e) => fertig({ code: 1, text: String(e) }));
  });
}

/** Summiert alle „test result: … N passed; M failed; K ignored“-Zeilen von cargo. */
export function rustSumme(text) {
  const s = { bestanden: 0, fehlgeschlagen: 0, uebersprungen: 0 };
  for (const m of text.matchAll(/test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored/g)) {
    s.bestanden += +m[1];
    s.fehlgeschlagen += +m[2];
    s.uebersprungen += +m[3];
  }
  return s;
}

/** Liest die Vitest-Zeile „Tests  78 passed (78)“ bzw. „Tests  2 failed | 76 passed (78)“. */
export function vitestSumme(text) {
  const z = [...text.matchAll(/^\s*Tests\s+(.+)$/gm)].pop()?.[1] ?? "";
  const zahl = (wort) => +(z.match(new RegExp(`(\\d+) ${wort}`))?.[1] ?? 0);
  return { bestanden: zahl("passed"), fehlgeschlagen: zahl("failed"), uebersprungen: zahl("skipped") + zahl("todo") };
}

const istHaupt = process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1]);
if (istHaupt) {
  const rust = await lauf("cargo", ["test", "--manifest-path", "src-tauri/Cargo.toml", "--", "--color", "always"]);
  const front = await lauf("npx", ["vitest", "run"]);
  const tsc = await lauf("npx", ["tsc", "--noEmit", "--pretty"]);

  const r = rustSumme(rust.text);
  const f = vitestSumme(front.text);
  const zeile = (name, s, code) =>
    `${name.padEnd(22)}${String(s.bestanden).padStart(4)} bestanden · ${s.fehlgeschlagen} fehlgeschlagen` +
    (s.uebersprungen ? ` · ${s.uebersprungen} übersprungen` : "") +
    (code !== 0 && s.fehlgeschlagen === 0 ? "  ⚠ Lauf abgebrochen" : "");
  const ok = rust.code === 0 && front.code === 0 && tsc.code === 0;
  const gruen = (t) => `\x1b[32m${t}\x1b[0m`;
  const rot = (t) => `\x1b[31m${t}\x1b[0m`;
  const farbe = (gut, t) => (gut ? gruen(t) : rot(t));

  console.log("\n══════════════ Testsumme ══════════════");
  console.log(farbe(rust.code === 0, zeile("Rust (cargo test)", r, rust.code)));
  console.log(farbe(front.code === 0, zeile("Frontend (Vitest)", f, front.code)));
  console.log(farbe(tsc.code === 0, `${"Typprüfung (tsc)".padEnd(22)}${tsc.code === 0 ? "   ok" : "   FEHLER"}`));
  console.log("───────────────────────────────────────");
  console.log(farbe(ok, `${"Gesamt".padEnd(22)}${String(r.bestanden + f.bestanden).padStart(4)} bestanden · ${r.fehlgeschlagen + f.fehlgeschlagen} fehlgeschlagen`));
  console.log(farbe(ok, ok ? "Ergebnis: GRÜN ✅" : "Ergebnis: ROT ❌"));
  process.exit(ok ? 0 : 1);
}
