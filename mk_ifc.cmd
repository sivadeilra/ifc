@echo off

rd /s/q out
mkdir out

rem cl /c input.cpp /std:c++latest /exportHeader /headerName:quote real_stuff.h /ifcOutput out

cl /c input.cpp /std:c++latest /exportHeader  /ifcOutput out
