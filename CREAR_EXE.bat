@echo off
title ROMForge Studio - crear instalador
cd /d "%~dp0"

echo.
echo  == ROMForge Studio ==
echo  Generando el instalador. La primera vez tarda varios minutos.
echo.

if not exist node_modules (
  echo  [1/2] Instalando dependencias...
  call npm install --no-audit --no-fund || goto :error
)

echo  [2/2] Compilando...
call npm run dist || goto :error

echo.
echo  Listo. El instalador esta en:
echo  src-tauri\target\release\bundle\nsis
echo.
explorer "src-tauri\target\release\bundle\nsis"
pause
exit /b 0

:error
echo.
echo  Fallo la compilacion. Revisa los mensajes de arriba.
pause
exit /b 1
