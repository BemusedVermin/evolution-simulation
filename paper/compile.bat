@echo off
REM ---------------------------------------------------------------------------
REM compile.bat - one-shot LaTeX -> PDF for channel_model.tex (Windows)
REM
REM Tries, in order:
REM   1. tectonic     (single self-contained binary; recommended)
REM   2. latexmk      (TeX Live / MiKTeX standard helper)
REM   3. pdflatex     (raw fallback; runs twice for cross-references)
REM ---------------------------------------------------------------------------

setlocal
cd /d "%~dp0"

set TEXFILE=channel_model.tex
set BASENAME=channel_model

echo.
echo === compiling %TEXFILE% ===
echo.

where tectonic >nul 2>&1
if %ERRORLEVEL%==0 (
    echo Using tectonic.
    tectonic %TEXFILE%
    goto :done
)

where latexmk >nul 2>&1
if %ERRORLEVEL%==0 (
    echo Using latexmk.
    latexmk -pdf -interaction=nonstopmode %TEXFILE%
    goto :done
)

where pdflatex >nul 2>&1
if %ERRORLEVEL%==0 (
    echo Using pdflatex (two passes for cross-references).
    pdflatex -interaction=nonstopmode %TEXFILE%
    pdflatex -interaction=nonstopmode %TEXFILE%
    goto :done
)

echo.
echo ERROR: no LaTeX engine found on PATH.
echo Install one of:
echo   - tectonic   (recommended, single binary): https://tectonic-typesetting.github.io
echo   - MiKTeX     (Windows-friendly TeX distribution): https://miktex.org
echo   - TeX Live   (cross-platform): https://tug.org/texlive
exit /b 1

:done
if exist %BASENAME%.pdf (
    echo.
    echo === compiled %BASENAME%.pdf ===
) else (
    echo.
    echo ERROR: compilation finished but %BASENAME%.pdf was not produced.
    echo Check the .log file for errors.
    exit /b 1
)

endlocal
