@ECHO OFF

RMDIR /S /Q Dist
MKDIR Dist

COPY Win32\Release\MuteApp.exe Dist\MuteApp-x86.exe
COPY x64\Release\MuteApp.exe Dist\MuteApp-x64.exe
