This .\frimware\ directory is the root directory for the code for RP2040 based macropad.
we would be using micropython for this project
GP0 -> COL1
GP1 -> COL2
GP2 -> COL3
GP3 -> ROW1
GP4 -> ROW2
GP5 -> ROW3

GP10 -> (indi switch (the extra switch which changes the layer of the keyboard for like presets))
and the other part of that indi switch is connected to ground.

the keyboard is
3X3 matrix keyboard with 1N4148 diodes
col---->switch----|diode>----row

and one 0.91 inch oled display
with its SDA in GP26 and SCL in GP27

THE DISPLAY WOULD SHOW WHICH LAYER IS THIS for now;

the macros can be changes using the command line gui
like the ui but in the terminal using emtypyie-cli (we would later update emtypyie-cli's cdn for it)
though make the commandline ui also (like that of opencode)
would ask the user to connect the macropad
then checks COM ports to identify it
then user confirms the COM port
and then it shows and asks user to add macros in the keys of each layer like they can keep some layers empty also it depends on the user
like they click the switch and then record macro (combination of some keys or some website hotkeys depends)
then flash and ready to goo
NAME OF THE MACROPAD IS Kohlrausch

use micropython and relevant skills for the frimware and comand line based ui