@echo off


cargo build --bin ifcdump
if errorlevel 1 exit /b


rd /s/q out
mkdir out

rem cl /c input.cpp /std:c++latest /exportHeader /headerName:quote real_stuff.h /ifcOutput out

rem cl /c input.cpp /std:c++latest /exportHeader  /ifcOutput out

cl /c base.ixx /std:c++20 /ifcOutput out
if errorlevel 1 exit /b

dir out

rem target\debug\ifcdump.exe out\base_mod.ifc > o.txt
target\debug\ifcdump.exe windows.h.ifc --functions --where rgn > o.txt
code o.txt
