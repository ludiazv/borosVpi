#!/bin/sh
echo "Writing Option bits to enable BEEP in PD4"
echo " ROP -> 0 ( no protection) "
echo " OPT1 & NOPT1 -> 00 FF (UBC default value)"
echo " OPT2 & NOPT2 -> 80 7F (enable AFR7 for beep peripherical)"
echo "Reset default..."
echo "0000ff00ff00ff00ff00ff" | xxd -r -p > rst.bin
xxd rst.bin
echo "0000ff807f" | xxd -r -p > beep_enable.bin
xxd beep_enable.bin
echo "done!"


