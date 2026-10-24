# Build the FastaGuard Application Note PDF with a standard TeX toolchain.
MAIN := main
LATEXMK ?= latexmk

.PHONY: all pdf clean distclean view

all: pdf

pdf:
	$(LATEXMK) -pdf -interaction=nonstopmode -halt-on-error $(MAIN).tex

# Fallback if latexmk is unavailable:
pdflatex-only:
	pdflatex -interaction=nonstopmode $(MAIN).tex
	bibtex $(MAIN)
	pdflatex -interaction=nonstopmode $(MAIN).tex
	pdflatex -interaction=nonstopmode $(MAIN).tex

clean:
	$(LATEXMK) -c $(MAIN).tex || true
	rm -f $(MAIN).bbl $(MAIN).blg $(MAIN).synctex.gz

distclean: clean
	rm -f $(MAIN).pdf

view: pdf
	@echo "Built $(MAIN).pdf"
