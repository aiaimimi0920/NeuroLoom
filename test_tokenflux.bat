@echo off
REM Root test script for TokenFlux provider

set API_KEY=sk-obWiEMnyPQrvdUJEQXxRgBQFORQmcEbhGxbNjmwxXWBnQOlWAxsEmtePPdwmfmbf

echo running chat test...
call crates\nl_llm_v2\examples\tokenflux\chat\test.bat "%API_KEY%" "What is 2+2?"

echo.
echo running stream test...
call crates\nl_llm_v2\examples\tokenflux\stream\test.bat "%API_KEY%" "Tell me a short joke."
