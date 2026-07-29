# Geminispace map

Downloads a gemini page, finds links, follow them, etc... Uses BFS algoritm, saves graph to json and dot.

I recommend using it with --cache enabled to not request other peoples capsules twice when re-running it. Cache stores page outlinks in json format and doesnt take a lot of space, sometimes even less than graph.

# Usage
```
Usage: geminispace-map [OPTIONS] <INPUT>

Arguments:
  <INPUT> (e.g. data/geminispace.json)

Options:
  -s, --start <START>            [default: gemini://gemini.circumlunar.space/capcom]
  -m, --max-depth <MAX_DEPTH>    [default: 5]
  -d, --dot-path <DOT_PATH>      
  -c, --cache                    
      --cache-path <CACHE_PATH>  [default: data/cache.json]
  -h, --help                     Print help
  -V, --version                  Print version
```

# Rendering graph (might take a while)
png
``` sh
sfdp geminispace.dot \
    -Tpng -o geminispace.png \
    -Goverlap=prism -Gsplines=line \
    -Gbgcolor="black" \
    -Nfontname="Arial" -Nfontsize=12 -Nfontcolor="white" -Ncolor="white" \
    -Ecolor="#ff0000" -Epenwidth=1 \
    -Gsep="+25" -GK=2 -Gmaxiter=50 -Gdpi=96 -v
```

png (without text, faster)
``` sh
sfdp geminispace.dot \
    -Tpng -o geminispace.png \
    -Goverlap=prism \
    -Gsplines=line \
    -Gbgcolor="black" \
    -Nlabel="" -Nshape=point -Ncolor="white" \
    -Ecolor="#ff0000" -Epenwidth=1 \
    -Gsep="+25" -GK=2 -Gmaxiter=50 -Gdpi=96 -v
```

svg
```
sfdp geminispace.dot \
    -Tsvg -o geminispace.svg \
    -Goverlap=prism -Gsplines=line \
    -Gbgcolor="black" \
    -Nfontname="Arial" -Nfontsize=8 -Nfontcolor="white" -Ncolor="white" \
    -Ecolor="#ff0000" -Epenwidth=1 \
    -Gsep="+25" -GK=2 -Gmaxiter=50 -v
```

For image viewer, i recommend vipsdisp, it doesnt crash on large images.

### Rendering issues
1. You might get a sfdp syntax error when a page url contains `\\\\`, you need to remove or replace them.
2. It may take a lot of time, on my mid tier pc it took 18 hours to render 80265 node graph png with text.
