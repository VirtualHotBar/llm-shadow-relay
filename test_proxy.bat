@echo off
cd /d E:\Temp\llm-shadow-relay
start /B target\release\llm-shadow-relay.exe
timeout /t 3 /nobreak > nul
echo === Health ===
curl -s http://127.0.0.1:8080/health
echo.
echo === Chat ===
curl -s -X POST http://127.0.0.1:8080/v1/chat/completions -H "Content-Type: application/json" -d "{\"model\":\"deepseek-chat\",\"messages\":[{\"role\":\"user\",\"content\":\"Say hello in one word\"}],\"max_tokens\":20}"
echo.
echo DONE