# Geminispace map

## Usage
```
Usage: geminispace-map [OPTIONS] <INPUT>

Arguments:
  <INPUT>

Options:
  -m, --max-depth <MAX_DEPTH>  [default: 5]
  -d, --dot-path <DOT_PATH>    [default: ""]
```

## Rendering graph
``` sh
sfdp -Tpng data/geminispace.dot -o data/geminispace.png \
      -Goverlap=false -Gsplines=true \
      -Gbgcolor="black" \
      -Nfontcolor="white" -Ncolor="white" \
      -Ecolor="#ff000080" -Epenwidth=3 \
      -Gsep="+25" -GK=2
```

For image viewer, i recommend vipsdisp
