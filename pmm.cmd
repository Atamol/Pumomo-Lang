@echo off
rem cmd reads this file in the OEM codepage, so keep the message ASCII
set "PUMOMO=%~dp0target\release\pumomo.exe"
if not exist "%PUMOMO%" (
  echo not built yet: run "cargo build --release" first 1>&2
  exit /b 2
)
"%PUMOMO%" %*
