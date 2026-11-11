#!/bin/zsh
cd "$(dirname "$0")"
source ../../shared/python_finden.sh requirements.txt || { read -k1 "?Taste drücken …"; exit 1; }
exec "$PYTHON" katalog_gui.py
