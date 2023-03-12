@ECHO OFF

RMDIR /S /Q Dist
MKDIR Dist

COPY builds\Release\MuteApp.exe Dist\MuteApp.exe
