"""Struktur einer SQLite-Datenbank anzeigen (Work-Tool).

Zweck:
    Listet Tabellen, Spalten (Typ, Nullable, Primärschlüssel, Standardwert)
    und Indizes einer Datenbank auf. Schreibt nichts.

Ziel:
    Schneller Blick auf das tatsächliche Schema, z. B. zum Abgleich mit
    ../shared/schema.sql.

Aufruf:  python3 db_struktur.py [DATEI]   (Standard: ../Katalog-DB/Bildkatalog.db)
"""
import sqlite3
import os

def analyze_db(db_path):
    """Struktur einer SQLite-Datenbank auf der Konsole ausgeben.

    Parameter:
        db_path: Pfad der Datenbankdatei.

    Rückgabe:
        Keine. Fehlt die Datei, wird eine Meldung ausgegeben.
    """
    # 1. Prüfung, ob die Datei existiert
    if not os.path.isfile(db_path):
        print(f"❌ FATAL: Die Datei '{db_path}' wurde nicht gefunden.")
        print("Bitte stellen Sie sicher, dass der Pfad korrekt ist und die Datei existiert.")
        return

    print(f"🔍 Analyse der Datei: {db_path}")
    
    try:
        # 2. Datenbank öffnen im 'read-only' Modus
        # Optional: 'timeout=-1' sorgt dafür, dass der Zugriff blockiert wird, falls jemand anders schreibzugriff versucht.
        conn = sqlite3.connect(db_path, uri=True) 
        conn.execute("PRAGMA busy_timeout = -1;") # Blockierend bei Konflikten
        cursor = conn.cursor()

        print("\n=== DB-Struktur ===\n")

        # A) Liste der Tabellen
        print("Tabellen:")
        query_tables = "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;"
        cursor.execute(query_tables)
        tables = cursor.fetchall()
        
        if not tables:
            print("  ⚠️  Keine benutzerdefinierte Tabelle gefunden.")
        else:
            for table in tables:
                t_name = table[0]
                print(f"  - {t_name}")
            print("-" * 30 + "\n")

        # B) Details jeder Tabelle (Spalten, Typen, PKs)
        for t_name in [t[0] for t in tables]:
            print(f"📄 Tabelle: '{t_name}'")
            
            pragma_query = f"PRAGMA table_info('{t_name}');"
            cursor.execute(pragma_query)
            columns = cursor.fetchall()
            
            print("  Spalten:")
            for col in columns:
                col_id, col_name, col_type, nullable, pk, default_val = col
                # Logische Null-Bestimmung
                null_status = "No" if nullable else "Yes"
                
                pk_str = "PK" if pk == 1 else "-"
                default_str = str(default_val) if default_val is not None else "-"
                
                print(f"    | {col_name:25} | {col_type:15} | Nullable: {null_status:5} | PK: {pk_str} | Default: {default_str}")
            
            print("\n")

        # C) Indizes
        print("Indizes:")
        query_indices = "SELECT name, tbl_name, sql FROM sqlite_master WHERE type='index';"
        cursor.execute(query_indices)
        indices = cursor.fetchall()
        
        has_indices = False
        for idx in indices:
            i_name, tbl, i_sql = idx
            # Nur nicht-interne Indizes anzeigen
            if not i_name.startswith('sqlite_'):
                has_indices = True
                print(f"  - Index '{i_name}' (für {tbl}):")
                print(f"    SQL: {i_sql[:50]}...") # Nur erste 50 Zeichen, falls sehr lang
        if not has_indices:
            print("  (Keine benutzerdefinierten Indizes)")
        print("\n")

        print("✅ Analyse erfolgreich abgeschlossen.")

    except sqlite3.Error as e:
        print(f"❌ ERROR: Fehler beim Lesen der Datenbank: {e}")
    finally:
        # Verbindungs ordnungsgemäß schließen
        conn.close()
        print("Verbindung geschlossen.")

if __name__ == "__main__":
    # Aufruf: python3 db_struktur.py [DATEI]   (Standard: Katalog-DB/Bildkatalog.db)
    import sys
    standard = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "Katalog-DB", "Bildkatalog.db")
    db_filename = os.path.expanduser(sys.argv[1]) if len(sys.argv) > 1 else os.path.normpath(standard)

    analyze_db(db_filename)
