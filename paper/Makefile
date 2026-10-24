# Build and validate the FastaGuard Application Note PDF.
MAIN := main
LATEXMK ?= latexmk
BUILDDIR := build
BIORXIV_PDF := $(BUILDDIR)/fastaguard-biorxiv.pdf

.PHONY: all pdf check biorxiv clean distclean view

all: pdf

pdf:
	$(LATEXMK) -pdf -interaction=nonstopmode -halt-on-error $(MAIN).tex

check:
	python3 check_submission.py

biorxiv: pdf
	mkdir -p $(BUILDDIR)
	cp $(MAIN).pdf $(BIORXIV_PDF)
	@echo "Wrote $(BIORXIV_PDF)"

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
	rm -rf $(BUILDDIR)

view: pdf
	@echo "Built $(MAIN).pdf"
