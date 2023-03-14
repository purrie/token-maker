#!/usr/bin/env bash
echo Installing to "$HOME/.local/bin"
cp ./PROGRAM ~/.local/bin/
cp ./PROGRAM.desktop ~/.local/share/applications/
mkdir -p ~/.local/share/PROGRAM
cp -rfu ./data/* ~/.local/share/PROGRAM/
echo Installation completed
