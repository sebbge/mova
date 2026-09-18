@echo off
cd /d "%~dp0app"

docker compose up -d

start "bondi server" cmd /k "cd server && cargo watch -x run"

cd web
pnpm run dev
