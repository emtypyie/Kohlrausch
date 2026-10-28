@echo off
setlocal
set "APP=%~dp0..\src-tauri\target\release\kohlrausch.exe"
if not exist "%APP%" set "APP=%~dp0..\src-tauri\target\debug\kohlrausch.exe"
if not exist "%APP%" (
  echo Kohlrausch executable not found. Run cargo build first. 1>&2
  exit /b 1
)
"%APP%" %*
exit /b %ERRORLEVEL%
