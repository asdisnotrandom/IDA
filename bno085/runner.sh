#!/bin/bash

picotool uf2 convert target/thumbv8m.main-none-eabihf/debug/bno085 -t elf firmware.uf2
picotool load -x firmware.uf2
