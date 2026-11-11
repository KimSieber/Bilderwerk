# Bilderwerk – passendes Python für Work und Sweep finden (wird von den .command-Dateien geladen).
#
# Sucht ein Python ab Version 3.10 mit Tk (tkinter). Reihenfolge:
#   1. Umgebungsvariable BILDERWERK_PYTHON
#   2. Homebrew (/opt/homebrew, /usr/local): python3.14 … python3.10, python3
#   3. python3 im Suchpfad
# Ergebnis in $PYTHON. Fehlen Pakete aus requirements.txt, wird ein Hinweis ausgegeben.
#
# Aufruf in einer .command-Datei:  source "<Pfad>/python_finden.sh" "<requirements.txt>" || exit 1

_bw_pruefen() {
  "$1" -c 'import sys, tkinter; sys.exit(0 if sys.version_info >= (3, 10) else 1)' 2>/dev/null
}

PYTHON=""
_bw_kandidaten=()
[[ -n "$BILDERWERK_PYTHON" ]] && _bw_kandidaten+=("$BILDERWERK_PYTHON")
for _bw_basis in /opt/homebrew/bin /usr/local/bin; do
  for _bw_v in 3.14 3.13 3.12 3.11 3.10 ""; do
    _bw_kandidaten+=("$_bw_basis/python$([[ -n $_bw_v ]] && echo "$_bw_v" || echo 3)")
  done
done
_bw_pfad=$(command -v python3 2>/dev/null) && _bw_kandidaten+=("$_bw_pfad")

for _bw_k in "${_bw_kandidaten[@]}"; do
  if [[ -x "$_bw_k" ]] && _bw_pruefen "$_bw_k"; then
    PYTHON="$_bw_k"
    break
  fi
done

if [[ -z "$PYTHON" ]]; then
  echo "Bilderwerk: Kein passendes Python gefunden (benötigt: Python ab 3.10 mit Tk)."
  echo "Installation mit Homebrew:  brew install python@3.14 python-tk@3.14"
  echo "Oder Pfad vorgeben:          export BILDERWERK_PYTHON=/pfad/zu/python3"
  return 1 2>/dev/null || exit 1
fi

if [[ -n "$1" && -f "$1" ]]; then
  _bw_fehlt=""
  "$PYTHON" -c 'import PIL' 2>/dev/null || _bw_fehlt+=" Pillow"
  "$PYTHON" -c 'import numpy' 2>/dev/null || _bw_fehlt+=" numpy"
  if [[ -n "$_bw_fehlt" ]]; then
    echo "Hinweis: In $PYTHON fehlen Pakete:$_bw_fehlt"
    echo "  Homebrew-Python:  brew install pillow numpy"
    echo "  sonst:            $PYTHON -m pip install -r \"$1\""
  fi
fi
unset _bw_kandidaten _bw_basis _bw_v _bw_pfad _bw_k _bw_fehlt
