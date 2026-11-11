// Minimale Typen für Node-Bordmittel in Tests (ohne @types/node als Abhängigkeit).
declare module "node:fs" {
  export function readFileSync(pfad: string, kodierung: "utf8"): string;
}
