@echo off


cargo build --bin dump
if errorlevel 1 exit /b


rd /s/q out
mkdir out

rem cl /c input.cpp /std:c++latest /exportHeader /headerName:quote real_stuff.h /ifcOutput out

rem cl /c input.cpp /std:c++latest /exportHeader  /ifcOutput out

cl /c base.ixx /std:c++20 /ifcOutput out
if errorlevel 1 exit /b

dir out

target\debug\dump.exe out\base_mod.ifc > o.txt
code o.txt
