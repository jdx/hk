#!/usr/bin/env pwsh
#MISE description="Generate pkl/Builtins.pkl from all builtins/*.pkl files"
$ErrorActionPreference = "Stop"
python3 scripts/gen_builtins.py
