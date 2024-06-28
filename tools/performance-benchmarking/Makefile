TMP ?= ./tmp
PANDOC_EXTS ?= +auto_identifiers

HTML_SRC=$(wildcard src/html/*)
MD_SRC=$(wildcard src/md/*)

HTML_FROM_MD=$(patsubst src/md/%.md,$(TMP)/html_from_md/%.html,$(MD_SRC))

ALL_HTML=$(HTML_SRC) $(HTML_FROM_MD)

TEMPLATES=$(wildcard templates/*)

BOX_WHISKERS=$(shell grep -e '~include tmp/box_whisker' $(MD_SRC) | cut -d' ' -f2)

default: output/index.html

output/index.html: $(TMP)/index.html $(BOX_WHISKERS)
	mkdir -p $(dir $@)
	bin/preprocess < $< > $@~
	mv $@~ $@

$(TMP)/box_whisker/%.html: src/box_whisker/%.yaml bin/mk-graph
	mkdir -p $(dir $@)
	bin/mk-graph $@~ < $<
	mv $@~ $@

$(TMP)/index.html: $(MD_SRC) $(TEMPLATES)
	mkdir -p $(dir $@)
	pandoc \
	  --standalone \
	  --embed-resources \
	  --toc \
	  --metadata title="benchcomp Documentation" \
	  --template=templates/pandoc-template.html \
	  --highlight-style=pygments \
	  --from markdown$(PANDOC_EXTS) \
	  --to html \
	  < $< > $@~
	  mv $@~ $@
