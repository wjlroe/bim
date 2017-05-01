@echo off

call "C:\Program Files (x86)\Microsoft Visual Studio 14.0\VC\vcvarsall.bat" x64

IF NOT EXIST build mkdir build
pushd build
cl -FC -Zi -EHsc ..\src\main.cpp /SUBSYSTEM:CONSOLE
popd
