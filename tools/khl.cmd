@echo off
setlocal
set "APP=%~dp0..\target\release\kohlrausch.exe"
if not exist "%APP%" set "APP=%~dp0..\target\debug\kohlrausch.exe"
if not exist "%APP%" (
  echo Kohlrausch executable not found. Run cargo build first. 1>&2
  exit /b 1
)
if /I "%~1"=="--c" (
  "%APP%" --config %2 %3 %4 %5 %6 %7 %8 %9
  exit /b %ERRORLEVEL%
)
"%APP%" %*
exit /b %ERRORLEVEL%
