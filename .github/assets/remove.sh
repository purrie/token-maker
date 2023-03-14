#!/usr/bin/env bash
AB=$(which PROGRAM)
if [ $? -eq 0 ]; then
    echo Removed executable
    rm "$AB"
fi
if [ -f "$HOME/.local/share/applications/PROGRAM.desktop" ]; then
    echo Removed desktop entry
    rm "$HOME/.local/share/applications/PROGRAM.desktop"
fi
if [ -d "$HOME/.local/share/PROGRAM" ]; then
    echo Removed assets
    rm -rf "$HOME/.local/share/PROGRAM"
fi
