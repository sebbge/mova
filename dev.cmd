@echo off
cd /d "%~dp0"

if /i "%~1"=="reset" (
  echo databases will be re-deployed...
  docker compose down -v
)

docker compose up -d --wait

start "citymob server" cmd /k "cd lib\server && cargo watch -x run"

cd client\web
pnpm run dev
