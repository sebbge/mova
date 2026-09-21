@echo off
cd /d "%~dp0"

docker compose up -d

start "citymob server" cmd /k "cd lib\server && cargo watch -x run"

cd client\web
pnpm run dev
