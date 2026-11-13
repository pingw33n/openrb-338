#!/usr/bin/env python3
"""OpenRB-338 panel redesign mockup (task-179): the whole panel as one SVG of 624 x 510 panel
units, drawn as vector art only (the mod's splash is the one bitmap). README.md explains it.

    mock.py OUT.svg SPLASH.jpg [WINDOW_WIDTH]

SPLASH.jpg is the standard mod's splash (its path is written into the SVG). A WINDOW_WIDTH above
624 centers the panel in a wider window and draws the backdrop left and right of it. The labels
are <text> in Inter: render the SVG with ./render and ./fonts (README.md)."""
import math, os, sys

O = []
def a(s): O.append(s)

FONT = os.environ.get("ORB_FONT", "Inter")     # the static TTFs of Inter 4.1 (OFL), given to the renderer
FONT_SCALE = float(os.environ.get("ORB_FONT_SCALE", "1"))

def n(v):
    s = f"{v:.2f}".rstrip('0').rstrip('.')
    return s if s else '0'

def rect(x, y, w, h, rx=0, fill="#000", extra=""):
    a(f'<rect x="{n(x)}" y="{n(y)}" width="{n(w)}" height="{n(h)}" rx="{n(rx)}" fill="{fill}" {extra}/>')

def line(x1, y1, x2, y2, stroke, w=0.5, extra=""):
    a(f'<line x1="{n(x1)}" y1="{n(y1)}" x2="{n(x2)}" y2="{n(y2)}" stroke="{stroke}" stroke-width="{n(w)}" {extra}/>')

def circle(cx, cy, r, fill, extra=""):
    a(f'<circle cx="{n(cx)}" cy="{n(cy)}" r="{n(r)}" fill="{fill}" {extra}/>')

def esc(s): return s.replace('&', '&amp;').replace('<', '&lt;')

def text(x, y, s, size, fill, anchor="middle", weight="bold", ls=0, extra="", family=FONT):
    if family == FONT and FONT_SCALE != 1:
        y -= size * 0.36 * (1 - FONT_SCALE)      # keep the text's middle where it was
        size *= FONT_SCALE; ls *= FONT_SCALE
    a(f'<text x="{n(x)}" y="{n(y)}" font-family="{family}" font-size="{n(size)}" font-weight="{weight}" '
      f'fill="{fill}" text-anchor="{anchor}" letter-spacing="{n(ls)}" {extra}>{esc(s)}</text>')

def stext(x, y, s, size, fill="#edf1f3", anchor="middle", weight="500", ls=0.15, sh="#1b262c", so=0.75, d=0.45):
    """Label printed on the panel: a soft dark edge below-right."""
    text(x + d, y + d, s, size, sh, anchor, weight, ls, f'opacity="{so}"')
    text(x, y, s, size, fill, anchor, weight, ls)

def g(tr=None, extra=""):
    t = f' transform="{tr}"' if tr else ''
    a(f'<g{t} {extra}>')
def eg(): a('</g>')

# ---------------------------------------------------------------- defs
def lg(id, stops, x1=0, y1=0, x2=0, y2=1):
    s = ''.join(f'<stop offset="{o}" stop-color="{c}"/>' if len(st) == 2 else
                f'<stop offset="{st[0]}" stop-color="{st[1]}" stop-opacity="{st[2]}"/>'
                for st in stops for o, c in [st[:2]])
    a(f'<linearGradient id="{id}" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">{s}</linearGradient>')

def rg(id, stops, cx=0.5, cy=0.5, r=0.5, fx=None, fy=None):
    s = ''.join(f'<stop offset="{st[0]}" stop-color="{st[1]}"' + (f' stop-opacity="{st[2]}"' if len(st) > 2 else '') + '/>'
                for st in stops)
    f = f' fx="{fx}" fy="{fy}"' if fx is not None else ''
    a(f'<radialGradient id="{id}" cx="{cx}" cy="{cy}" r="{r}"{f}>{s}</radialGradient>')

def defs():
    a('<defs>')
    # blurs
    for id, sd in (("b05", 0.5), ("b1", 0.9), ("b2", 1.8), ("b3", 3)):
        a(f'<filter id="{id}" x="-60%" y="-60%" width="220%" height="220%"><feGaussianBlur stdDeviation="{sd}"/></filter>')
    # grain: speckles of black / white, clipped to the source shape
    def grain(id, freq, octaves, seed, ch, k, off, white):
        c = 1 if white else 0
        row = ['0'] * 5
        row[ch] = str(k); row[4] = str(off)
        a(f'<filter id="{id}" x="0" y="0" width="1" height="1" color-interpolation-filters="sRGB">'
          f'<feTurbulence type="fractalNoise" baseFrequency="{freq}" numOctaves="{octaves}" seed="{seed}"/>'
          f'<feColorMatrix type="matrix" values="0 0 0 0 {c}  0 0 0 0 {c}  0 0 0 0 {c}  {" ".join(row)}"/>'
          f'<feComposite in2="SourceGraphic" operator="in"/></filter>')
    grain("grD", "1.3", 2, 3, 0, 2.6, -1.25, False)
    grain("grL", "1.3", 2, 11, 1, 2.6, -1.3, True)
    grain("blotD", "0.09", 3, 5, 0, 2.2, -1.0, False)
    grain("blotL", "0.07", 3, 21, 2, 2.2, -1.05, True)
    grain("woodD", "0.9 0.012", 3, 4, 0, 2.4, -0.95, False)
    grain("woodL", "1.4 0.02", 2, 9, 1, 2.4, -1.15, True)
    grain("marble", "0.035 0.05", 4, 8, 0, 2.6, -1.2, False)
    # section plates
    lg("gHdrPat", [(0, "#c23148"), (0.5, "#a82439"), (1, "#8a1a2c")])
    lg("gHdrMix", [(0, "#b8b055"), (0.5, "#a39b41"), (1, "#878033")])
    lg("gHdrFx", [(0, "#3d62b8"), (0.5, "#2c4b9c"), (1, "#213a80")])
    lg("gSheen", [(0, "#fff", 0.10), (0.5, "#fff", 0), (1, "#000", 0.14)])
    lg("g303top", [(0, "#f6f6f6"), (0.6, "#e3e4e5"), (1, "#cfd0d2")])
    lg("g303bezel", [(0, "#dcdddf"), (1, "#b9babd")])
    lg("gKeyW", [(0, "#f2f2f2"), (0.85, "#ffffff"), (1, "#d9d9d9")])
    lg("gKeyB", [(0, "#2a2a2a"), (1, "#0c0c0c")])
    lg("g808", [(0, "#4a473d"), (1, "#3a372f")])
    lg("g909", [(0, "#efeee9"), (1, "#dddbd3")])
    lg("g909hdr", [(0, "#5b5c5c"), (1, "#3c3d3d")])
    # silver button faces
    lg("gSilver", [(0, "#fbfbfc"), (0.45, "#d5d8db"), (1, "#a4a8ad")])
    lg("gSilverP", [(0, "#9da1a6"), (1, "#d0d3d6")])
    lg("gShadeTop", [(0, "#000", 0.6), (0.4, "#000", 0.22), (1, "#000", 0)])
    lg("gShadeLeft", [(0, "#000", 0.5), (0.4, "#000", 0.16), (1, "#000", 0)], 0, 0, 1, 0)
    lg("gPillLit", [(0, "#ffffff"), (0.5, "#e6f2ff"), (1, "#b9d9fb")], 0, 0, 1, 1)
    lg("gPillFlatLit", [(0, "#eaecee"), (1, "#cacdd1")], 0, 0, 1, 1)
    lg("gPillFlatSoft", [(0, "#dadcdf"), (1, "#b9bcc1")], 0, 0, 1, 1)
    lg("gPillP", [(0, "#777b80"), (0.5, "#a3a7ab"), (1, "#c4c7cb")], 0, 0, 1, 1)
    lg("gPill", [(0, "#f4f5f6"), (0.5, "#c3c6ca"), (1, "#8e9297")], 0, 0, 1, 1)
    lg("gCap", [(0, "#ffffff"), (0.55, "#e4e6e8"), (1, "#b8bcc1")])
    lg("gCapLit", [(0, "#fff6f3"), (0.5, "#ffc9bf"), (1, "#ff8c7c")])
    lg("gTrans", [(0, "#f1f3f4"), (0.1, "#d9dee0"), (0.2, "#aeb6ba"), (0.75, "#8c969b"), (1, "#69737a")])
    lg("gFader", [(0, "#6b7077"), (0.2, "#3b3f45"), (0.46, "#17191c"), (0.54, "#17191c"), (0.8, "#34383d"), (1, "#16181b")])
    lg("gSlot", [(0, "#050506"), (0.6, "#131416"), (1, "#3d4349")], 0, 0, 1, 0)
    # knobs
    rg("gKnobBody", [(0, "#4a4e55"), (0.7, "#25282c"), (1, "#0d0e10")], 0.4, 0.35, 0.75)
    lg("gKnobCap", [(0, "#b9c1d0"), (0.5, "#858da0"), (1, "#4f5665")], 0.15, 0.1, 0.85, 0.9)
    lg("g303Wall", [(0, "#8f9398"), (0.45, "#4a4d51"), (1, "#17181a")], 0.15, 0.1, 0.85, 0.9)
    lg("g303Edge", [(0, "#ffffff"), (0.5, "#d0d2d5"), (1, "#5c5f63")], 0.15, 0.1, 0.85, 0.9)
    a('<clipPath id="k303c"><circle cx="0" cy="0" r="8.5"/></clipPath>')
    lg("g303Skirt", [(0, "#ffffff"), (0.5, "#d2d4d7"), (1, "#84878c")], 0.15, 0.1, 0.85, 0.9)
    rg("g303Cap", [(0, "#ffffff"), (0.45, "#e9eaec"), (0.85, "#b4b7bb"), (1, "#93969b")], 0.42, 0.38, 0.62)
    lg("gGrayCollar", [(0, "#3a3d42"), (0.5, "#141517"), (1, "#020203")], 0.15, 0.1, 0.85, 0.9)
    lg("gGrayTop", [(0, "#949aab"), (1, "#6a7083")])
    lg("g808RedF", [(0, "#f6473a"), (1, "#d41f15")])
    lg("g808WhiteF", [(0, "#f8fdff"), (1, "#d3e4ea")])
    lg("g808Collar", [(0, "#2c2c2a"), (0.5, "#0d0d0c"), (1, "#000000")], 0.15, 0.1, 0.85, 0.9)
    lg("g909Wall", [(0, "#4a4a47"), (1, "#141413")], 0.15, 0.1, 0.85, 0.9)
    lg("g909Top", [(0, "#5a5a56"), (1, "#343431")], 0.15, 0.1, 0.85, 0.9)
    lg("gRimLight", [(0, "#fff", 0.55), (0.5, "#fff", 0.05), (1, "#000", 0.35)], 0.15, 0.1, 0.85, 0.9)
    rg("g808Red", [(0, "#ff7a66"), (0.35, "#ee2f22"), (0.85, "#b0140d"), (1, "#6e0a06")], 0.4, 0.35, 0.7)
    rg("g808White", [(0, "#ffffff"), (0.5, "#f1f6f8"), (0.9, "#b9c4ca"), (1, "#8c979d")], 0.4, 0.35, 0.7)
    rg("g909Knob", [(0, "#6a6a67"), (0.6, "#3a3a38"), (1, "#191918")], 0.4, 0.35, 0.7)
    # step keys of the 808
    for name, c in (("R", ("#ff6a55", "#e8372b", "#a8140e")), ("O", ("#ffb65a", "#f58a21", "#c9600c")),
                    ("Y", ("#fff27a", "#f3dc3c", "#c4a916")), ("W", ("#ffffff", "#f2f2ee", "#c4c5c0"))):
        lg("g808k" + name, [(0, c[0]), (0.28, c[1]), (0.34, c[0]), (0.6, c[1]), (1, c[2])])
    # LEDs
    def led(id, hi, mid, lo):
        rg(id, [(0, hi), (0.45, mid), (1, lo)], 0.4, 0.35, 0.65)
    led("ledR", "#ffd9d2", "#ff3b2c", "#b3140c")
    led("ledRoff", "#8a3a34", "#5a1512", "#2a0605")
    led("ledG", "#eaffe0", "#43e03a", "#12861a")
    led("ledGoff", "#3f6a43", "#1f4426", "#0d2311")
    led("ledY", "#fffbd0", "#ffd42e", "#b98a08")
    led("ledYoff", "#7a6a35", "#4f431c", "#261f0a")
    rg("haloR", [(0, "#ff4a36", 0.75), (1, "#ff4a36", 0)])
    rg("haloG", [(0, "#5cff4a", 0.7), (1, "#5cff4a", 0)])
    rg("haloY", [(0, "#ffd640", 0.7), (1, "#ffd640", 0)])
    rg("haloO", [(0, "#ff9a30", 0.7), (1, "#ff9a30", 0)])
    lg("gWood", [(0, "#5a3b2e"), (0.25, "#8a5d47"), (0.6, "#7a503c"), (1, "#4e3227")], 0, 0, 1, 0)
    lg("gLever", [(0, "#8a9197"), (0.35, "#ffffff"), (0.7, "#b9c0c6"), (1, "#5e656b")], 0, 0, 1, 0)
    lg("gLogoInk", [(0, "#ffffff"), (1, "#c5d0d7")])
    lg("gLogoOr", [(0, "#ffb347"), (1, "#f07a12")])
    lg("gScreen", [(0, "#000", 0.55), (0.12, "#000", 0), (1, "#000", 0)])
    a('</defs>')

# ---------------------------------------------------------------- primitives
def plate(x, y, w, h, fill, hi=0.22, lo=0.45):
    """A section plate: face, light top/left edge, dark bottom/right edge, the dark seam around."""
    rect(x, y, w, h, 0, "#14181b")
    rect(x + 0.5, y + 0.5, w - 1, h - 1, 0.8, fill)

def plate_edges(x, y, w, h, hi=0.28, lo=0.45):
    a(f'<path d="M{n(x+0.9)} {n(y+h-1)} V{n(y+0.9)} H{n(x+w-1)}" fill="none" stroke="#fff" stroke-opacity="{hi}" stroke-width="0.6"/>')
    a(f'<path d="M{n(x+w-0.9)} {n(y+1.2)} V{n(y+h-0.9)} H{n(x+1.2)}" fill="none" stroke="#000" stroke-opacity="{lo}" stroke-width="0.6"/>')

def led_round(cx, cy, r, color, on):
    """color: R, G, Y"""
    if on:
        circle(cx, cy, r * 2.6, f"url(#halo{color})")
    circle(cx, cy, r + 0.45, "#0b0b0b", 'opacity="0.85"')
    circle(cx, cy, r, f"url(#led{color}{'' if on else 'off'})")
    if on:
        circle(cx - r * 0.3, cy - r * 0.35, r * 0.3, "#fff", 'opacity="0.8"')

SEG = {'0': 'abcdef', '1': 'bc', '2': 'abged', '3': 'abgcd', '4': 'fgbc', '5': 'afgcd', '6': 'afgedc',
       '7': 'abc', '8': 'abcdefg', '9': 'abcdfg', ' ': ''}

def seg_paths(x, y):
    """The seven segments of a 9 x 16 digit cell at (x, y)."""
    l, r, t, m, b, k = x + 1.7, x + 7.3, y + 2.0, y + 8.0, y + 14.0, 0.85
    def hz(yy): return f'M{n(l+0.5)} {n(yy)} L{n(l+1.3)} {n(yy-k)} H{n(r-1.3)} L{n(r-0.5)} {n(yy)} L{n(r-1.3)} {n(yy+k)} H{n(l+1.3)} Z'
    def vt(xx, y0, y1): return f'M{n(xx)} {n(y0+0.5)} L{n(xx+k)} {n(y0+1.3)} V{n(y1-1.3)} L{n(xx)} {n(y1-0.5)} L{n(xx-k)} {n(y1-1.3)} V{n(y0+1.3)} Z'
    return {'a': hz(t), 'g': hz(m), 'd': hz(b), 'f': vt(l, t, m), 'b': vt(r, t, m), 'e': vt(l, m, b), 'c': vt(r, m, b)}

def display(x, y, w, h, value, enabled=True):
    digits = int(w // 9)
    rect(x - 0.8, y - 0.8, w + 1.6, h + 1.6, 0.8, "#0a0a0a")
    rect(x, y, w, h, 0.4, "#1c0505")
    s = str(value).rjust(digits) if enabled else ' ' * digits
    x0 = x + (w - 9 * digits) / 2
    off, on = [], []
    for i, ch in enumerate(s):
        p = seg_paths(x0 + 9 * i, y + (h - 16) / 2)
        for k, d in p.items():
            (on if k in SEG[ch] else off).append(d)
    g(f"translate({n(x + w / 2)} {n(y + h / 2)}) skewX(-5) translate({n(-x - w / 2)} {n(-y - h / 2)})")
    a(f'<path d="{" ".join(off)}" fill="#4a0f0c"/>')
    if on:
        a(f'<path d="{" ".join(on)}" fill="#ff3020" filter="url(#b05)" opacity="0.7"/>')
        a(f'<path d="{" ".join(on)}" fill="#ff5545"/>')
    eg()
    rect(x, y, w, h * 0.45, 0.4, "#fff", 'opacity="0.05"')

def arrows(x, y):
    """Up at (x, y), down at (x, y + 9), each 11 x 9."""
    rect(x - 0.4, y - 0.4, 11.8, 18.8, 0.9, "#0d0d0d")
    for i in (0, 1):
        yy = y + 9 * i
        rect(x + 0.7, yy + 0.7, 9.6, 7.6, 0.7, "url(#gSilver)")
        rect(x + 1.1, yy + 1.0, 8.8, 0.6, 0.3, "#fff", 'opacity="0.85"')
        cx = x + 5.5
        if i == 0:
            a(f'<path d="M{n(cx-2.4)} {n(yy+6)} L{n(cx)} {n(yy+2.6)} L{n(cx+2.4)} {n(yy+6)} Z" fill="#24272b"/>')
        else:
            a(f'<path d="M{n(cx-2.4)} {n(yy+3)} L{n(cx)} {n(yy+6.4)} L{n(cx+2.4)} {n(yy+3)} Z" fill="#24272b"/>')

def knob_ticks(cx, cy, r0, r1, count, stroke, w, opacity=1.0):
    d = []
    for i in range(count):
        ang = math.radians(-135 + 270 * i / (count - 1))
        s, c = math.sin(ang), -math.cos(ang)
        d.append(f'M{n(cx + r0 * s)} {n(cy + r0 * c)} L{n(cx + r1 * s)} {n(cy + r1 * c)}')
    a(f'<path d="{" ".join(d)}" stroke="{stroke}" stroke-width="{n(w)}" stroke-linecap="round" opacity="{opacity}" fill="none"/>')

def knob_gray(x, y, v, ticks=True):
    """The 18 x 19 knob of the gray sections: a flat blue-gray cap in a black knurled collar,
    seen from above, with a cast shadow."""
    cx, cy = x + 9, y + 9.5
    if ticks:
        knob_ticks(cx, cy, 9.4, 11.6, 7, "#e6ecef", 0.7, 0.9)
    R, T = 7.7, 5.75
    g(f"translate({n(cx)} {n(cy)})")
    a(f'<path d="M0 0 L1.4 1.7" stroke="#000" stroke-width="{n(2*R+1.4)}" stroke-linecap="round" opacity="0.10"/>')
    a(f'<path d="M0 0 L1.4 1.7" stroke="#000" stroke-width="{n(2*R)}" stroke-linecap="round" opacity="0.30"/>')
    circle(0, 0, R, "url(#gGrayCollar)", 'stroke="#050506" stroke-width="0.35"')
    circle(0, 0, (R + T) / 2 + 0.1, "none", f'stroke="#8a9098" stroke-opacity="0.35" stroke-width="{n(R-T-0.5)}" stroke-dasharray="0.4 0.62"')
    circle(0, 0, T, "url(#gGrayTop)")
    circle(0, 0, T - 0.2, "none", 'stroke="url(#gRimLight)" stroke-width="0.4"')
    ang = -135 + 270 * v
    a(f'<line x1="0" y1="-0.7" x2="0" y2="{n(-T+0.55)}" stroke="#fff" stroke-width="1.1" stroke-linecap="round" transform="rotate({n(ang)})"/>')
    eg()

def wedge(r, mid, half):
    """A pie slice of radius r around the origin; angles in degrees, 0 = up, clockwise."""
    a0, a1 = math.radians(mid - half), math.radians(mid + half)
    return (f'M0 0 L{n(r*math.sin(a0))} {n(-r*math.cos(a0))} A{n(r)} {n(r)} 0 0 1 '
            f'{n(r*math.sin(a1))} {n(-r*math.cos(a1))} Z')

def knob_303(x, y, v):
    """A machined aluminum cylinder seen from above: a cast shadow for its height, a dark knurled
    wall, and a flat top whose brushed finish reflects in sectors (not a sphere's round highlight).
    The body is static; only the pointer turns."""
    cx, cy = x + 15, y + 15
    knob_ticks(cx, cy, 11.9, 14.5, 11, "#26282a", 1.25)
    R, T = 10.0, 8.5      # wall, top face
    g(f"translate({n(cx)} {n(cy)})")
    # the cast shadow: the body swept away from the light
    a(f'<path d="M0 0 L1.7 2.1" stroke="#000" stroke-width="{n(2*R+1.4)}" stroke-linecap="round" opacity="0.09"/>')
    a(f'<path d="M0 0 L1.7 2.1" stroke="#000" stroke-width="{n(2*R)}" stroke-linecap="round" opacity="0.24"/>')
    # wall and knurling
    circle(0, 0, R, "url(#g303Wall)", 'stroke="#141516" stroke-width="0.45"')
    circle(0, 0, (R + T) / 2, "none", f'stroke="#e8eaec" stroke-opacity="0.55" stroke-width="{n(R-T-0.3)}" stroke-dasharray="0.42 0.6"')
    # the flat top
    circle(0, 0, T, "#d9dbde")
    a('<g clip-path="url(#k303c)"><g filter="url(#b1)">'
      f'<path d="{wedge(13, 315, 34)} {wedge(13, 135, 30)}" fill="#ffffff"/>'
      f'<path d="{wedge(13, 45, 26)} {wedge(13, 225, 28)}" fill="#878b90" opacity="0.62"/>'
      '</g></g>')
    circle(0, 0, T, "none", 'stroke="url(#g303Edge)" stroke-width="0.55"')
    circle(0, 0, T - 0.75, "none", 'stroke="#000" stroke-opacity="0.10" stroke-width="0.35"')
    circle(0, 0, 0.9, "#8b8e93", 'opacity="0.5"')
    ang = -135 + 270 * v
    a(f'<line x1="0" y1="-1.6" x2="0" y2="{n(-T+0.5)}" stroke="#151617" stroke-width="1.4" transform="rotate({n(ang)})"/>')
    eg()

def knob_808(x, y, v, red):
    """A flat colored cap in a black collar, seen from above, with a cast shadow."""
    cx, cy = x + 6.5, y + 6.5
    R, T = 5.75, 4.7
    g(f"translate({n(cx)} {n(cy)})")
    a(f'<path d="M0 0 L1.3 1.6" stroke="#000" stroke-width="{n(2*R+1.6)}" stroke-linecap="round" opacity="0.16"/>')
    a(f'<path d="M0 0 L1.3 1.6" stroke="#000" stroke-width="{n(2*R)}" stroke-linecap="round" opacity="0.42"/>')
    circle(0, 0, R, "url(#g808Collar)")
    circle(0, 0, T, "url(#g808RedF)" if red else "url(#g808WhiteF)")
    circle(0, 0, T - 0.2, "none", 'stroke="url(#gRimLight)" stroke-width="0.4"')
    ang = -135 + 270 * v
    a(f'<line x1="0" y1="-2.1" x2="0" y2="{n(-T-0.1)}" stroke="{"#210403" if red else "#101112"}" stroke-width="1.2" transform="rotate({n(ang)})"/>')
    eg()

def knob_909(x, y, v):
    """A dark flat-topped cylinder with an orange index near its edge."""
    cx, cy = x + 6, y + 6
    knob_ticks(cx, cy, 5.7, 7.0, 11, "#4b4b48", 0.5, 0.9)
    R, T = 5.25, 4.55
    g(f"translate({n(cx)} {n(cy)})")
    a(f'<path d="M0 0 L1.1 1.4" stroke="#000" stroke-width="{n(2*R+1.2)}" stroke-linecap="round" opacity="0.08"/>')
    a(f'<path d="M0 0 L1.1 1.4" stroke="#000" stroke-width="{n(2*R)}" stroke-linecap="round" opacity="0.25"/>')
    circle(0, 0, R, "url(#g909Wall)", 'stroke="#0c0c0b" stroke-width="0.3"')
    circle(0, 0, T, "url(#g909Top)")
    circle(0, 0, T - 0.15, "none", 'stroke="url(#gRimLight)" stroke-width="0.35" opacity="0.7"')
    ang = -135 + 270 * v
    a(f'<line x1="0" y1="-1.9" x2="0" y2="{n(-T+0.15)}" stroke="#ff7a1c" stroke-width="1.25" transform="rotate({n(ang)})"/>')
    eg()

def fader(x, y, v, ticks="#d5dde2"):
    """Track 15 x 54 at (x, y), handle 15 x 11."""
    for i in range(11):
        yy = y + 5.5 + 4.3 * i
        line(x + 0.8, yy, x + 14.2, yy, ticks, 0.5, 'opacity="0.8"')
        line(x + 0.8, yy + 0.5, x + 14.2, yy + 0.5, "#000", 0.4, 'opacity="0.25"')
    rect(x + 6.4, y + 1.5, 2.2, 51, 1.1, "url(#gSlot)")
    hy = y + (1 - v) * 43
    rect(x + 1.6, hy + 2.2, 14, 11, 1.4, "#000", 'opacity="0.45" filter="url(#b1)"')
    rect(x + 0.4, hy, 14.2, 11, 1.3, "url(#gFader)", 'stroke="#050506" stroke-width="0.4"')
    rect(x + 1.1, hy + 0.5, 12.8, 0.6, 0.3, "#fff", 'opacity="0.35"')
    rect(x + 0.9, hy + 5.05, 13.2, 0.9, 0.2, "#e9edf0")
    for dy in (2.3, 3.6, 7.6, 8.9):
        rect(x + 1.3, hy + dy, 12.4, 0.35, 0, "#000", 'opacity="0.55"')

PRESS_STYLE = "flatlit"  # how a pressed button is drawn: flat, travel, lit, sunk, reversed

def pill(x, y, w, h, pressed=False, r=None, style=None):
    """A small silver push button in a dark well, at rest or pressed (in one of the styles)."""
    r = min(w, h) * 0.22 if r is None else r
    style = style or PRESS_STYLE
    fr0 = max(r - 0.6, 0.3)
    def rest(dx=0.0, shadow=True, hi=0.9):
        if shadow:
            rect(x + 0.6, y + 0.9, w, h, r, "#000", 'opacity="0.35" filter="url(#b05)"')
        rect(x, y, w, h, r, "#1f2022")
        rect(x + 0.8 + dx, y + 0.8 + dx, w - 1.6, h - 1.6, fr0, "url(#gPill)")
        a(f'<path d="M{n(x+1.2+dx)} {n(y+h-1.3+dx)} V{n(y+1.2+dx)} H{n(x+w-1.3+dx)}" fill="none" stroke="#fff" stroke-opacity="{hi}" stroke-width="0.5"/>')
    if not pressed:
        rest()
    elif style == "flat":
        # the original's way: the bevel goes, the face is plain and has moved one pixel down-right
        rect(x, y, w, h, r, "#1f2022")
        rect(x + 1.2, y + 1.2, w - 1.7, h - 1.7, fr0, "#c6c9cd")
        rect(x + 1.55, y + 1.55, w - 2.4, h - 2.4, max(fr0 - 0.3, 0.2), "none", 'stroke="#fff" stroke-opacity="0.4" stroke-width="0.35"')
    elif style == "travel":
        # the same button, moved down-right into its well: no cast shadow, a dimmer bevel
        rect(x, y, w, h, r, "#1f2022")
        a(f'<clipPath id="pt{n(x)}_{n(y)}"><rect x="{n(x+0.5)}" y="{n(y+0.5)}" width="{n(w-1)}" height="{n(h-1)}" rx="{n(fr0)}"/></clipPath>')
        g(None, f'clip-path="url(#pt{n(x)}_{n(y)})"')
        rect(x + 1.4, y + 1.4, w - 1.6, h - 1.6, fr0, "url(#gPill)")
        a(f'<path d="M{n(x+1.8)} {n(y+h-0.7)} V{n(y+1.8)} H{n(x+w-0.7)}" fill="none" stroke="#fff" stroke-opacity="0.55" stroke-width="0.5"/>')
        eg()
    elif style == "lit":
        # lights up while held, the iPad version's way
        rect(x - 0.8, y - 0.8, w + 1.6, h + 1.6, r + 0.8, "#a9d4ff", 'opacity="0.7" filter="url(#b05)"')
        rect(x, y, w, h, r, "#1f2022")
        rect(x + 0.8, y + 0.8, w - 1.6, h - 1.6, fr0, "url(#gPillLit)")
        a(f'<path d="M{n(x+1.2)} {n(y+h-1.3)} V{n(y+1.2)} H{n(x+w-1.3)}" fill="none" stroke="#fff" stroke-opacity="0.9" stroke-width="0.5"/>')
    elif style in ("flatlit", "flatsoft"):
        # flat, but not one color: the bevel and the cast shadow go, a faint gradient stays.
        # flatlit: in place and lighter (C without the blue); flatsoft: moved 1 px, rest brightness.
        d = 0.0 if style == "flatlit" else 0.4
        rect(x, y, w, h, r, "#1f2022")
        rect(x + 0.8 + d, y + 0.8 + d, w - 1.6 - d, h - 1.6 - d, fr0,
             "url(#gPillFlatLit)" if style == "flatlit" else "url(#gPillFlatSoft)")
        rect(x + 1.15 + d, y + 1.15 + d, w - 2.3 - d, h - 2.3 - d, max(fr0 - 0.3, 0.2), "none",
             'stroke="#000" stroke-opacity="0.10" stroke-width="0.35"')
    elif style == "reversed":
        rect(x, y, w, h, r, "#1f2022")
        rect(x + 0.8, y + 0.8, w - 1.6, h - 1.6, fr0, "url(#gPillP)")
        a(f'<path d="M{n(x+1.3)} {n(y+h-1.2)} V{n(y+1.3)} H{n(x+w-1.2)}" fill="none" stroke="#000" stroke-opacity="0.55" stroke-width="0.9"/>')
        a(f'<path d="M{n(x+1.6)} {n(y+h-1.1)} H{n(x+w-1.1)} V{n(y+1.6)}" fill="none" stroke="#fff" stroke-opacity="0.45" stroke-width="0.4"/>')
    else:   # sunk
        rect(x, y, w, h, r, "#111213")
        fx, fy, fw, fh = x + 1.5, y + 1.5, w - 2.1, h - 2.1
        fr = max(r - 0.9, 0.3)
        rect(fx, fy, fw, fh, fr, "url(#gPill)")
        rect(fx, fy, fw, fh, fr, "#1a1c1f", 'opacity="0.34"')
        rect(fx, fy, fw, min(fh * 0.55, 3.4), fr, "url(#gShadeTop)")
        rect(fx, fy, min(fw * 0.35, 3.0), fh, fr, "url(#gShadeLeft)")
        a(f'<path d="M{n(x+1.2)} {n(y+h-0.45)} H{n(x+w-0.45)} V{n(y+1.2)}" fill="none" stroke="#fff" stroke-opacity="0.28" stroke-width="0.4"/>')

def keycap(cx, cy, size, label, lit=False, fs=7.4, dark=False):
    s = size
    x, y = cx - s / 2, cy - s / 2
    if lit:
        rect(x - 1.0, y - 1.0, s + 2.0, s + 2.0, 3.2, "#ff3a2a", 'opacity="0.8" filter="url(#b1)"')
    rect(x, y, s, s, 2.4, "url(#gCapLit)" if lit else "url(#gCap)",
         f'stroke="{"#e53b2d" if lit else "#5d6166"}" stroke-width="{"0.6" if lit else "0.3"}"')
    rect(x + 1.3, y + 1.0, s - 2.6, s * 0.36, 1.8, "#fff", f'opacity="{0.55 if lit else 0.7}"')
    if label:
        text(cx, cy + fs * 0.36, label, fs, "#a8251b" if lit else "#3c4046", "middle", "bold")

def slide_v(x, y, w, h, pos, light=False):
    """A vertical two-position slide switch; pos 0 = top."""
    rect(x, y, w, h, 1.2, "#060606", 'stroke="#000" stroke-width="0.3"')
    rect(x + 0.5, y + 0.5, w - 1, h - 1, 0.9, "#1a1a1a")
    th = h * 0.5
    ty = y + 0.8 if pos == 0 else y + h - th - 0.8
    c0, c1 = ("#d9dcdf", "#8d9196") if light else ("#5a5d61", "#2b2d30")
    a(f'<linearGradient id="sv{n(x)}_{n(y)}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{c0}"/><stop offset="1" stop-color="{c1}"/></linearGradient>')
    rect(x + 0.9, ty, w - 1.8, th, 0.8, f"url(#sv{n(x)}_{n(y)})", 'stroke="#000" stroke-width="0.3"')
    for i in range(3):
        yy = ty + th * (0.28 + 0.22 * i)
        line(x + 1.7, yy, x + w - 1.7, yy, "#000", 0.45, 'opacity="0.6"')
        line(x + 1.7, yy + 0.45, x + w - 1.7, yy + 0.45, "#fff", 0.3, 'opacity="0.3"')

def lever(cx, cy, down):
    """A chrome bat toggle seen from above."""
    a(f'<ellipse cx="{n(cx)}" cy="{n(cy)}" rx="3.3" ry="4.1" fill="#0b0c0d" stroke="#8b949a" stroke-width="0.5"/>')
    dy = 5.6 if down else -5.6
    y0, y1 = sorted((cy, cy + dy))
    a(f'<ellipse cx="{n(cx+0.9)}" cy="{n(cy+dy*0.55+1.2)}" rx="2.2" ry="4.4" fill="#000" opacity="0.4" filter="url(#b05)"/>')
    rect(cx - 1.75, y0 - 1.6, 3.5, y1 - y0 + 3.2, 1.75, "url(#gLever)", 'stroke="#33383c" stroke-width="0.35"')
    circle(cx, cy + dy, 1.9, "url(#gLever)", 'stroke="#33383c" stroke-width="0.35"')

def onoff(x, y, w, h, on, frame="#3a0d14"):
    """The green on/off LED button of the headers."""
    cx, cy = x + w / 2, y + h / 2
    s = min(w, h) - 4.4
    if on:
        circle(cx, cy, s * 1.25, "url(#haloG)")
    rect(cx - s / 2 - 1.3, cy - s / 2 - 1.3, s + 2.6, s + 2.6, 1.4, frame)
    rect(cx - s / 2 - 0.5, cy - s / 2 - 0.5, s + 1, s + 1, 0.9, "#0c1a0e")
    rect(cx - s / 2, cy - s / 2, s, s, 0.7, "url(#ledG)" if on else "url(#ledGoff)")
    if on:
        rect(cx - s / 2 + 0.7, cy - s / 2 + 0.6, s - 1.4, s * 0.32, 0.5, "#fff", 'opacity="0.55"')

def meter4(x, y, w, h, level):
    """The 4-segment input meter of a header; level 0..4 lit from the bottom."""
    rect(x + 1.5, y + 1, w - 3, h - 2, 0.8, "#101010")
    cols = [("#ff3a2c", "#4a120e"), ("#ffb02e", "#4a3410"), ("#d8f23a", "#3c4312"), ("#48e83c", "#12401a")]
    sh = (h - 4.4) / 4
    for i in range(4):
        on = (3 - i) < level
        yy = y + 2.2 + i * sh
        if on:
            rect(x + 1.6, yy - 0.6, w - 3.2, sh + 0.4, 1, cols[i][0], 'opacity="0.5" filter="url(#b05)"')
        rect(x + 3, yy, w - 6, sh - 0.9, 0.3, cols[i][0] if on else cols[i][1])

ROUTE_STYLE = "led"      # None: the key follows PRESS_STYLE; "led": the key stays up, only the LED lights

def route_switch(x, y, on, style=None):
    """The 24 x 11 routing key with its LED (Dist / PCF / Comp)."""
    style = style or ROUTE_STYLE
    if style == "led":
        # a momentary key: it never stays down, the lamp alone shows the state
        pill(x + 0.5, y + 1.2, 15.5, 8.6, pressed=False, r=1.6)
    else:
        pill(x + 0.5, y + 1.2, 15.5, 8.6, pressed=on, r=1.6, style=style)
    led_round(x + 20.3, y + 5.6, 1.9, "Y", on)

# ---------------------------------------------------------------- sections
GRAY = "#5f7079"

def gray_base():
    # the gray plates (grain is laid over all of them at once)
    plate(16, 0, 592, 50, GRAY)
    for r in range(4):
        plate(16, 50 + 115 * r, 70, 115, GRAY)
        plate(456, 50 + 115 * r, 70, 115, GRAY)
    for (y, h) in ((50, 98), (148, 111), (259, 93), (352, 65), (417, 93)):
        plate(526, y, 82, h, GRAY)

def gray_rects():
    r = [(16, 0, 592, 50)]
    for i in range(4):
        r += [(16, 50 + 115 * i, 70, 115), (456, 50 + 115 * i, 70, 115)]
    r += [(526, 50, 82, 98), (526, 148, 82, 111), (526, 259, 82, 93), (526, 352, 82, 65), (526, 417, 82, 93)]
    return r

def grain_over(rects, id, layers):
    a(f'<clipPath id="{id}">' + ''.join(f'<rect x="{x+0.5}" y="{y+0.5}" width="{w-1}" height="{h-1}" rx="0.8"/>' for x, y, w, h in rects) + '</clipPath>')
    x0 = min(r[0] for r in rects); y0 = min(r[1] for r in rects)
    x1 = max(r[0] + r[2] for r in rects); y1 = max(r[1] + r[3] for r in rects)
    g(None, f'clip-path="url(#{id})"')
    for f, op in layers:
        rect(x0, y0, x1 - x0, y1 - y0, 0, "#000", f'filter="url(#{f})" opacity="{op}"')
    eg()

def wood(x):
    rect(x, 0, 16, 510, 0, "url(#gWood)")
    a(f'<clipPath id="wc{x}"><rect x="{x}" y="0" width="16" height="510"/></clipPath>')
    g(None, f'clip-path="url(#wc{x})"')
    rect(x, 0, 16, 510, 0, "#000", 'filter="url(#woodD)" opacity="0.55"')
    rect(x, 0, 16, 510, 0, "#000", 'filter="url(#woodL)" opacity="0.28"')
    eg()
    rect(x, 0, 1, 510, 0, "#000", 'opacity="0.45"')
    rect(x + 15, 0, 1, 510, 0, "#000", 'opacity="0.55"')
    rect(x + 1, 0, 0.6, 510, 0, "#fff", 'opacity="0.12"')

def knob_scale(kx, ky, lo, hi, title):
    """The printed scale and title of an 18 x 19 knob at (kx, ky)."""
    stext(kx - 2.6, ky + 24.2, lo, 5.4, weight="500")
    stext(kx + 20.8, ky + 24.2, hi, 5.4, weight="500")
    stext(kx + 9, ky + 31.2, title, 6.2)

def header(x, y, w, h, grad, title, tx, size=9.0, ls=0.4):
    rect(x + 0.5, y + 0.8, w, h, 1.2, "#000", 'opacity="0.45" filter="url(#b05)"')
    rect(x, y, w, h, 1.1, f"url(#{grad})")
    rect(x, y, w, h, 1.1, "url(#gSheen)")
    rect(x + 0.6, y + 0.4, w - 1.2, 0.6, 0.3, "#fff", 'opacity="0.35"')
    text(tx + 0.5, y + h / 2 + size * 0.36 + 0.5, title, size, "#000", "middle", "bold", ls, 'opacity="0.45"')
    text(tx, y + h / 2 + size * 0.36, title, size, "#f7f7f2", "middle", "bold", ls)

def transport():
    g("translate(16 0)")
    plate_edges(0, 0, 592, 50)
    # shuffle
    knob_gray(11, 13, 0.35)
    stext(8, 37.6, "0", 5.4); stext(32.4, 37.6, "10", 5.4)
    stext(20, 45.8, "SHUFFLE", 5.6, weight="bold", ls=0.1)
    # sync / midi (MIDI is out of scope: dark)
    led_round(44.5, 7.5, 1.9, "G", False); stext(50, 10.2, "Sync", 7.2, anchor="start", weight="500", fill="#c9d3d8")
    led_round(79.5, 7.5, 1.9, "G", False); stext(85, 10.2, "MIDI", 7.2, anchor="start", weight="500", fill="#c9d3d8")
    # tempo
    stext(44, 24.2, "Tempo", 8.6, anchor="start", weight="bold", ls=0)
    display(44, 27, 27, 16, 140)
    arrows(73, 26)
    # mode switch
    stext(158, 13.4, "Pattern", 9.2, anchor="end", weight="500", ls=0)
    led_round(165.5, 11.5, 1.9, "G", False)
    lever(174, 11.5, True)
    led_round(182.5, 11.5, 1.9, "Y", True)
    stext(192, 13.4, "Song mode", 9.2, anchor="start", weight="500", ls=0)
    # transport keys
    rect(95.4, 20.2, 162.2, 27.2, 2.2, "#000", 'opacity="0.5" filter="url(#b1)"')
    rect(94.8, 19.8, 162.4, 27.4, 2.0, "#0c0d0e")
    for i in range(5):
        x = 96 + 32 * i
        rect(x + 0.6, 21.6, 30.8, 23.8, 1.0, "url(#gTrans)")
        rect(x + 1.0, 22.0, 30.0, 1.0, 0.5, "#fff", 'opacity="0.8"')
        rect(x + 0.6, 43.4, 30.8, 2.0, 0.8, "#000", 'opacity="0.25"')
        cx, cy = x + 16, 34
        if i == 0:   # play, lit
            circle(cx, cy, 8, "url(#haloG)")
            a(f'<path d="M{n(cx-3.6)} {n(cy-4.6)} L{n(cx+4.8)} {n(cy)} L{n(cx-3.6)} {n(cy+4.6)} Z" fill="#35e22a" stroke="#0e6a12" stroke-width="0.4" stroke-linejoin="round"/>')
        elif i == 1:
            rect(cx - 3.2, cy - 3.2, 6.4, 6.4, 0.5, "#8a3b12", 'stroke="#3c1806" stroke-width="0.4"')
        elif i == 2:
            a(f'<path d="M{n(cx-0.4)} {n(cy-3.6)} V{n(cy+3.6)} L{n(cx-6)} {n(cy)} Z M{n(cx+5.4)} {n(cy-3.6)} V{n(cy+3.6)} L{n(cx-0.2)} {n(cy)} Z" fill="#4b545a"/>')
        elif i == 3:
            a(f'<path d="M{n(cx+0.4)} {n(cy-3.6)} V{n(cy+3.6)} L{n(cx+6)} {n(cy)} Z M{n(cx-5.4)} {n(cy-3.6)} V{n(cy+3.6)} L{n(cx+0.2)} {n(cy)} Z" fill="#4b545a"/>')
        else:
            circle(cx, cy, 3.5, "#9a1b14", 'stroke="#4a0a07" stroke-width="0.4"')
            circle(cx - 1, cy - 1.1, 1.0, "#ff8a7a", 'opacity="0.5"')
    # bar
    stext(276, 24.2, "Bar", 8.6, anchor="start", weight="bold", ls=0)
    display(271, 27, 27, 16, 9); arrows(300, 26)
    # loop
    lever(321.5, 12, True); led_round(329.2, 12.3, 1.7, "Y", True)
    stext(362.5, 12.4, "Loop", 9.2, weight="500", ls=0)
    for (x1, x2) in ((334.5, 348), (377.5, 400)):
        line(x1 + 0.4, 9.9, x2 + 0.4, 9.9, "#1b262c", 0.7, 'opacity="0.7"')
        line(x1, 9.5, x2, 9.5, "#edf1f3", 0.7)
    stext(331, 24.2, "Start", 8.6, anchor="start", weight="bold", ls=0)
    display(329, 27, 27, 16, 1); arrows(358, 26)
    stext(372.5, 24.2, "Length", 8.6, anchor="start", weight="bold", ls=0)
    display(372, 27, 27, 16, 16); arrows(401, 26)
    if DRAW_LOGO:
        logo(458, 1)
    eg()

LOGO_OPEN = ['<path transform="translate(0)" d="M2 0 A2 3 0 1 0 2 6 A2 3 0 1 0 2 0 Z"/>',
             '<path transform="translate(5.4)" d="M0 6 V0 H2.5 A1.5 1.7 0 0 1 2.5 3.4 H0"/>',
             '<path transform="translate(10.8)" d="M3.6 0 H0 V6 H3.6 M0 3 H2.8"/>',
             '<path transform="translate(15.8)" d="M0 6 V0 L4 6 V0"/>',
             '<path transform="translate(21.2)" d="M0 6 V0 H2.5 A1.5 1.7 0 0 1 2.5 3.4 H0 M2 3.4 L4 6"/>',
             '<path transform="translate(26.6)" d="M0 0 V6 M0 0 H2.4 A1.1 1.5 0 0 1 2.4 3 H0 M2.4 3 H2.9 A1.1 1.5 0 0 1 2.9 6 H0"/>',
             '<path transform="translate(32)" d="M0 3 H2.6"/>']
THREE = "M0.3 1 C0.8 0.3 1.3 0 2 0 C3 0 3.6 0.6 3.6 1.5 C3.6 2.4 2.9 3 1.8 3 C3 3 3.8 3.6 3.8 4.5 C3.8 5.4 3 6 1.9 6 C1.1 6 0.5 5.7 0.1 5"
EIGHT = "M1.9 3 C0.9 3 0.3 2.4 0.3 1.5 C0.3 0.6 0.9 0 1.9 0 C2.9 0 3.5 0.6 3.5 1.5 C3.5 2.4 2.9 3 1.9 3 C0.8 3 0.1 3.7 0.1 4.5 C0.1 5.4 0.8 6 1.9 6 C3 6 3.7 5.4 3.7 4.5 C3.7 3.7 3 3 1.9 3 Z"
LOGO_338 = [f'<path transform="translate(0)" d="{THREE}"/>', f'<path transform="translate(5.2)" d="{THREE}"/>',
            f'<path transform="translate(10.4)" d="{EIGHT}"/>']

OPEN_TRAVEL = 49.466667     # logo_anim.rs
RB_X = 8.0 + OPEN_TRAVEL
DRAW_LOGO = True

def logo_frame(t):
    """logo_anim.rs Frame::at: (open_dx, [seg alpha x4], sub alpha, icon alpha)."""
    ramp = lambda t, t0, d: min(max((t - t0) / d, 0.0), 1.0)
    ease = lambda u: 1.0 - (1.0 - u) ** 3
    return (OPEN_TRAVEL * (1.0 - ease(ramp(t, 0.0, 1.0))),
            [ramp(t, 0.55 + 0.20 * i, 0.25) for i in range(4)],
            ramp(t, 1.35, 0.5), ramp(t, 0.0, 0.6))

def logo_piece(name):
    """One piece of the wordmark on the 134 x 48 logo canvas (the pieces of LOGO_PIECES): the
    glyph paths of the current art, pressed into the plate (a dark lower edge, a light upper
    edge, a heavier stroke)."""
    def run(tx, paths, stroke, sw=1.02):
        for dx, dy, col, op, w in ((0.55, 0.75, "#0b1216", 0.75, sw + 0.25), (-0.3, -0.35, "#ffffff", 0.35, sw)):
            a(f'<g transform="translate({n(tx+dx)} {n(9.5+dy)}) scale(2.33)" fill="none" stroke="{col}" stroke-opacity="{op}" stroke-width="{w}" stroke-linecap="round" stroke-linejoin="round">{"".join(paths)}</g>')
        a(f'<g transform="translate({tx} 9.5) scale(2.33)" fill="none" stroke="{stroke}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round">{"".join(paths)}</g>')
    if name == "open":
        run(8, LOGO_OPEN[:4], "#f2f5f7")
    elif name == "rb338":
        run(8, LOGO_OPEN[4:], "#f2f5f7")
        run(92, LOGO_338, "#ff9a2e")
    elif name.startswith("line_"):
        # the underline: four bars in the 808's step colors, as glossy as its keys
        i = int(name[5:]); c = "ROYW"[i]
        bx = 8 + 29.9 * i
        rect(bx + 0.4, 28.2, 28.4, 2.6, 1.0, "#000", 'opacity="0.55" filter="url(#b05)"')
        rect(bx, 27.6, 28.4, 2.6, 0.9, f"url(#g808k{c})", 'stroke="#101010" stroke-width="0.3"')
    elif name == "sub":
        stext(8, 40.4, "BASS LINE AND DRUM MACHINE", 5.5, anchor="start", weight="500", ls=1.02, fill="#d7dfe4")

LOGO_PIECES = ("rb338", "open", "line_0", "line_1", "line_2", "line_3", "sub")
_clip_n = [0]

def logo(x, y, t=None, scale=1.0):
    """The wordmark at (x, y); `t` seconds into the startup animation, or the final state."""
    g(f"translate({n(x)} {n(y)}) scale({n(scale)})")
    if t is None:
        for p in LOGO_PIECES:
            logo_piece(p)
    else:
        dx, seg, sub, _ = logo_frame(t)
        logo_piece("rb338")
        _clip_n[0] += 1
        cid = f"lgm{_clip_n[0]}"
        a(f'<clipPath id="{cid}"><rect x="0" y="0" width="{n(RB_X)}" height="48"/></clipPath>')
        g(None, f'clip-path="url(#{cid})"'); g(f"translate({n(dx)} 0)"); logo_piece("open"); eg(); eg()
        for i in range(4):
            if seg[i] > 0:
                g(None, f'opacity="{n(seg[i])}"'); logo_piece(f"line_{i}"); eg()
        if sub > 0:
            g(None, f'opacity="{n(sub)}"'); logo_piece("sub"); eg()
    eg()

def pattern_section(r, pattern, bank, on=True, focus=False, shuffle=False):
    y0 = 50 + 115 * r
    g(f"translate(16 {y0})")
    plate_edges(0, 0, 70, 115)
    header(4, 4, 62, 13.6, "gHdrPat", "PATTERN", 39.5, 8.4, 0.35)
    onoff(5, 4, 13, 13.6, on)
    # pattern keys
    rect(3.5, 22.8, 63, 31.8, 3.0, "#0d0e0f")
    rect(3.5, 22.8, 63, 31.8, 3.0, "none", 'stroke="#fff" stroke-opacity="0.18" stroke-width="0.35"')
    for i in range(8):
        cx = 11.6 + 15.6 * (i % 4); cy = 30.9 + 15.6 * (i // 4)
        keycap(cx, cy, 14.6, str(i + 1), lit=(i == pattern))
    stext(5, 63.0, "BANK:", 6.0, anchor="start", weight="bold")
    rect(3.5, 64.8, 63, 16.2, 3.0, "#0d0e0f")
    rect(3.5, 64.8, 63, 16.2, 3.0, "none", 'stroke="#fff" stroke-opacity="0.18" stroke-width="0.35"')
    for i in range(4):
        keycap(11.6 + 15.6 * i, 72.9, 14.6, "ABCD"[i], lit=(i == bank))
    stext(5, 91.6, "SHUFFLE", 5.4, anchor="start", weight="bold", ls=0)
    stext(36.5, 91.6, "STEPS", 5.4, anchor="start", weight="bold", ls=0.2)
    rect(3.7, 93.9, 16.6, 16.6, 3.0, "#0d0e0f")
    rect(3.7, 93.9, 16.6, 16.6, 3.0, "none", 'stroke="#fff" stroke-opacity="0.18" stroke-width="0.35"')
    keycap(12, 102.2, 15.0, "", lit=shuffle)
    display(36, 95, 19, 16, 16)
    arrows(56, 94)
    eg()
    # focus bar
    rect(86, y0, 4, 115, 0, "#101214")
    rect(87.2, y0 + 1, 1.8, 113, 0.6, "#ff3b30" if focus else "#4a1512")
    if focus:
        rect(86.6, y0 + 1, 3, 113, 1, "#ff3b30", 'opacity="0.55" filter="url(#b05)"')

def note_icon(x, y, s=1.0):
    g(f"translate({n(x)} {n(y)}) scale({s})")
    a('<ellipse cx="0" cy="0" rx="2.5" ry="1.8" transform="rotate(-22)" fill="#151515"/>')
    a('<path d="M2.1 -0.6 V-11.6" stroke="#151515" stroke-width="0.9" fill="none"/>')
    a('<path d="M2.1 -11.6 C3 -8.6 6.2 -8.2 5.2 -4.6 C5.4 -6.8 3.4 -7.6 2.1 -8.2 Z M2.1 -8.4 C3 -5.6 6 -5.2 5 -1.8 C5.2 -3.8 3.4 -4.6 2.1 -5.2 Z" fill="#151515"/>')
    eg()

def rest_icon(x, y):
    g(f"translate({n(x)} {n(y)})")
    a('<path d="M3.6 -6 L0.2 6" stroke="#151515" stroke-width="1.0" stroke-linecap="round" fill="none"/>')
    a('<path d="M3.5 -5.4 C2.4 -3.6 0.6 -3.4 -0.6 -4.2 M2.3 -1.6 C1.2 0.2 -0.6 0.4 -1.8 -0.4" stroke="#151515" stroke-width="0.8" fill="none" stroke-linecap="round"/>')
    circle(-0.6, -4.9, 1.25, "#151515"); circle(-1.8, -1.1, 1.25, "#151515")
    eg()

def tb303(idx, knobs, wave, step, note, flags, pitch=False, pressed=()):
    y0 = 50 + 115 * idx
    g(f"translate(90 {y0})")
    rect(0, 0, 366, 115, 0, "#14181b")
    # the knob plate
    rect(0.5, 0.5, 365, 45, 1.0, "url(#g303top)")
    a('<path d="M1 45 V1 H365" fill="none" stroke="#fff" stroke-width="0.7" opacity="0.9"/>')
    a('<path d="M365.2 1.4 V45.2 H1.4" fill="none" stroke="#000" stroke-width="0.6" opacity="0.3"/>')
    lab = dict(size=7.2, fill="#1b1c1e", weight="bold", ls=0.45)
    text(32.65, 8.4, "WAVEFORM", 6.7, lab['fill'], "middle", "bold", 0.4)
    a('<path d="M6 29 V20 L13 29 V20 L20 29" fill="none" stroke="#1b1c1e" stroke-width="1.05" stroke-linejoin="miter"/>')
    a('<path d="M45.3 29 V20 H49.3 V29 H53.3 V20 H57.3 V29 H59.3" fill="none" stroke="#1b1c1e" stroke-width="1.05" stroke-linejoin="miter"/>')
    # waveform switch (horizontal slide)
    rect(24.65, 18.5, 16, 12, 1.0, "#080808")
    tx = 25.55 if wave == 0 else 32.75
    rect(tx, 19.4, 7, 10.2, 0.7, "#3a3c3f", 'stroke="#000" stroke-width="0.3"')
    for i in range(3):
        xx = tx + 1.8 + 1.7 * i
        line(xx, 20.5, xx, 28.5, "#000", 0.5, 'opacity="0.7"'); line(xx + 0.45, 20.5, xx + 0.45, 28.5, "#fff", 0.3, 'opacity="0.25"')
    line(65.3, 6, 65.3, 40, "#2a2b2d", 0.7); line(65.9, 6, 65.9, 40, "#fff", 0.5, 'opacity="0.8"')
    line(274.7, 6, 274.7, 40, "#2a2b2d", 0.7); line(275.3, 6, 275.3, 40, "#fff", 0.5, 'opacity="0.8"')
    for i, name in enumerate(("TUNE", "CUTOFF", "RESO", "ENV.MOD", "DECAY", "ACCENT")):
        text(87 + 33 * i, 8.4, name, 6.7, "#1b1c1e", "middle", "bold", 0.3)
        knob_303(72 + 33 * i, 12, knobs[i])
    text(339.5, 13.2, "EDIT", 6.7, "#1b1c1e", "middle", "bold", 0.4)
    text(339.5, 21.2, "STEP", 6.7, "#1b1c1e", "middle", "bold", 0.4)
    display(330, 25, 19, 16, step)
    # the key plate: a silver bezel around an inset frame
    rect(0.5, 46.2, 365, 68.3, 1.0, "url(#g303bezel)")
    a('<path d="M1 114 V46.8 H365" fill="none" stroke="#fff" stroke-width="0.7" opacity="0.85"/>')
    a('<path d="M365.2 47 V114.3 H1.4" fill="none" stroke="#000" stroke-width="0.6" opacity="0.35"/>')
    rect(6.6, 48.4, 352.2, 61.6, 1.2, "#f5f5f5", 'stroke="#8e9195" stroke-width="0.5"')
    rect(8.0, 49.4, 349.4, 59.4, 0.5, "#101010")
    X0, Y0, Y1 = 8.6, 50.0, 108.2
    # pitch mode / clear
    rect(X0, Y0, 36.4, 37, 0, "#0b0b0b")
    text(27, 58.4, "PITCH", 6.6, "#f2f2f2", "middle", "bold", 0.3)
    text(27, 65.0, "MODE", 6.6, "#f2f2f2", "middle", "bold", 0.3)
    led_round(27, 70, 1.8, "R", pitch)
    pill(19.5, 73.6, 15, 8.4)
    rect(X0, 87.4, 36.4, Y1 - 87.4, 0, "url(#gKeyW)")
    text(27, 94.2, "CLEAR", 6.6, "#1b1c1e", "middle", "bold", 0.3)
    pill(18, 96, 17, 10)
    # keyboard
    for i in range(8):
        kx = 46 + 21 * i
        rect(kx + 0.35, Y0, 20.3, Y1 - Y0, 0, "url(#gKeyW)")
        a(f'<path d="M{n(kx+0.35)} {n(Y1-2)} V{n(Y1)} H{n(kx+20.65)} V{n(Y1-2)}" fill="#000" opacity="0.10"/>')
    whites = [0, 2, 4, 5, 7, 9, 11, 12]
    for bx, sem in ((67, 1), (88, 3), (130, 6), (151, 8), (172, 10)):
        a(f'<path d="M{n(bx-7)} {n(Y0)} H{n(bx+7)} V{n(79.4)} Q{n(bx+7)} {n(81.6)} {n(bx+4.8)} {n(81.6)} H{n(bx-4.8)} Q{n(bx-7)} {n(81.6)} {n(bx-7)} {n(79.4)} Z" fill="url(#gKeyB)"/>')
        a(f'<path d="M{n(bx+7.5)} {n(Y0)} V{n(80)}" stroke="#000" stroke-opacity="0.18" stroke-width="1"/>')
        led_round(bx, 59, 1.8, "R", note == sem)
        pill(bx - 4.2, 62.6, 8.4, 15.4, r=1.2)
    for i, sem in enumerate(whites):
        kx = 52 + 21 * i
        led_round(kx + 4.5, 85, 1.9, "R", note == sem)
        pill(kx, 89, 9, 16, r=1.2)
    # note / rest cells
    rect(215.2, Y0, 51.2, 20.6, 0, "url(#gKeyW)")
    rect(267.2, Y0, 49.6, 20.6, 0, "url(#gKeyW)")
    note_icon(231, 66.6, 0.95)
    led_round(250, 63, 1.9, "R", 'note' in flags)
    rest_icon(277, 61.6)
    led_round(288, 63, 1.9, "R", 'rest' in flags)
    pill(294, 58, 17, 10)
    for i, name in enumerate(("DOWN", "UP", "ACCENT", "SLIDE")):
        cx0 = 215.2 + 25.4 * i
        text(cx0 + 12.7, 77.8, name, 5.9, "#f2f2f2", "middle", "bold", 0.15)
        rect(cx0, 80.2, 24.8, Y1 - 80.2, 0, "url(#gKeyW)")
        bx = 224 + 25 * i
        led_round(bx + 4.5, 85, 1.9, "R", name.lower() in flags)
        pill(bx, 89, 9, 16, r=1.2)
    # back / step
    rect(317.6, Y0, 39.2, 28.8, 0, "url(#gKeyW)")
    rect(317.6, 79.6, 39.2, Y1 - 79.6, 0, "url(#gKeyW)")
    text(337.2, 58.8, "BACK", 6.6, "#1b1c1e", "middle", "bold", 0.3)
    pill(328, 62, 17, 10, pressed='back' in pressed)
    text(337.2, 87.6, "STEP", 6.6, "#1b1c1e", "middle", "bold", 0.3)
    pill(324.6, 90, 25.4, 17, pressed='step' in pressed, r=1.6)
    a(f'<path d="M331.4 91.4 V105.6" stroke="#000" stroke-opacity="0.25" stroke-width="0.5"/><path d="M343.4 91.4 V105.6" stroke="#fff" stroke-opacity="0.7" stroke-width="0.5"/>')
    eg()

CREAM = "#efe9c6"

def chip808(x, y, w, h, label, sel=False):
    rect(x + 0.5, y + 0.7, w, h, 1.6, "#000", 'opacity="0.5" filter="url(#b05)"')
    rect(x, y, w, h, 1.6, "#fbf6d6" if sel else CREAM, 'stroke="#8a8462" stroke-width="0.3"')
    rect(x + 0.8, y + 0.5, w - 1.6, h * 0.35, 1.0, "#fff", 'opacity="0.45"')
    if sel:
        rect(x - 0.9, y - 0.9, w + 1.8, h + 1.8, 2.3, "none", 'stroke="#ff8a1e" stroke-width="1.1"')
        rect(x - 0.9, y - 0.9, w + 1.8, h + 1.8, 2.3, "none", 'stroke="#ff8a1e" stroke-width="1.6" opacity="0.5" filter="url(#b05)"')
    text(x + w / 2, y + h / 2 + 2.45, label, 6.8, "#25231a", "middle", "bold", 0.25)

def tr808(steps, selected, splash, mod_name):
    g("translate(90 280)")
    rect(0, 0, 366, 115, 0, "#14181b")
    rect(0.5, 0.5, 365, 114, 1.0, "url(#g808)")
    eg()
    grain_over([(90, 280, 366, 115)], "c808", (("grD", 0.5), ("grL", 0.10), ("blotD", 0.14)))
    g("translate(90 280)")
    a('<path d="M1 114 V1 H365" fill="none" stroke="#fff" stroke-width="0.6" opacity="0.16"/>')
    a('<path d="M365.2 1.4 V114.2 H1.4" fill="none" stroke="#000" stroke-width="0.6" opacity="0.5"/>')
    for i in range(13):
        x = 9.5 + 22 * i
        line(x, 4.5, x, 70, "#b9b49a", 0.45, 'opacity="0.55"')
        line(x + 0.45, 4.5, x + 0.45, 70, "#000", 0.4, 'opacity="0.5"')
    lab = "#d8d3b6"
    names = ["AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CB", "CY", "OH", "CH"]
    lev = [0.62, 0.78, 0.66, 0.7, 0.7, 0.7, 0.72, 0.68, 0.64, 0.7, 0.66, 0.7]
    for i in range(12):
        cx = 20.5 + 22 * i
        text(cx, 7.9, "LEVEL", 5.3, lab, "middle", "500", 0.1)
        knob_808(14 + 22 * i, 9, lev[i], True)
    for i, name in ((1, "TONE"), (2, "TUNE"), (3, "TUNE"), (4, "TUNE"), (5, "TUNE"), (9, "TONE")):
        text(20.5 + 22 * i, 29.6, name, 5.3, lab, "middle", "500", 0.1)
        knob_808(14 + 22 * i, 31, 0.5, False)
    for i, name in ((1, "DECAY"), (2, "SNAPPY"), (9, "DECAY"), (10, "DECAY")):
        text(20.5 + 22 * i, 52.3, name, 5.0 if name == "SNAPPY" else 5.3, lab, "middle", "500", 0 if name == "SNAPPY" else 0.1)
        knob_808(14 + 22 * i, 54, [0.45, 0.55, 0.3, 0.6][(i * 7) % 4], False)
    for k, name in enumerate(("LC", "MC", "HC", "CL", "MA")):
        chip808(78.6 + 22 * k, 44.4, 17.8, 8.8, name)
        slide_v((81, 103, 125, 146, 169)[k], 55, 11, 17, 1 if k < 3 else 0)
    for i in range(12):
        chip808(11.6 + 22 * i, 72.6, 17.8, 9.2, names[i], sel=(i == selected))
    # step keys
    for i in range(16):
        x = 16 + 16 * i
        c = "ROYW"[i // 4]
        rect(x + 0.8, 91.2, 12.6, 18.6, 1.2, "#000", 'opacity="0.6" filter="url(#b05)"')
        rect(x, 90.2, 12.6, 19, 1.0, f"url(#g808k{c})", 'stroke="#0b0b0a" stroke-width="0.45"')
        rect(x + 0.7, 90.8, 11.2, 0.6, 0.3, "#fff", 'opacity="0.6"')
        on = i in steps
        led_round(x + 6.3, 93.4, 1.15, "R", on)
    # the mod's splash, where the original has the instrument selector
    rect(277.6, 4.6, 84.4, 106, 2.2, "#000", 'opacity="0.55"')
    rect(277.2, 4.2, 84.4, 106, 2.2, "#16150f", 'stroke="#000" stroke-width="0.5"')
    a('<path d="M278 109.6 H361 V5" fill="none" stroke="#fff" stroke-opacity="0.14" stroke-width="0.5"/>')
    rect(280.9, 7.4, 77, 77, 0.8, "#000")
    a(f'<image x="281.4" y="7.9" width="76" height="76" href="{splash}" preserveAspectRatio="xMidYMid slice"/>')
    rect(281.4, 7.9, 76, 76, 0, "url(#gScreen)")
    line(282, 89.4, 357, 89.4, "#fff", 0.4, 'opacity="0.12"')
    text(319.4, 101.2, mod_name, 6.6, "#e8e2c2", "middle", "500", 0.15)
    eg()

def tr909(steps, selected, flam=False):
    g("translate(90 395)")
    rect(0, 0, 366, 115, 0, "#14181b")
    rect(0.5, 0.5, 365, 114, 1.4, "url(#g909)")
    eg()
    grain_over([(90, 395, 366, 115)], "c909", (("marble", 0.07), ("grD", 0.08)))
    g("translate(90 395)")
    a('<path d="M1 114 V1 H365" fill="none" stroke="#fff" stroke-width="0.8" opacity="0.95"/>')
    a('<path d="M365.2 1.4 V114.2 H1.4" fill="none" stroke="#000" stroke-width="0.7" opacity="0.4"/>')
    rect(2.4, 2.4, 361.2, 110.2, 1.0, "none", 'stroke="#8f8f8a" stroke-width="0.4" opacity="0.7"')
    ink = "#3a3a37"
    names = ["AC", "BD", "SD", "LT", "MT", "HT", "RS", "CP", "CH", "OH", "CC", "RC"]
    def chip(x, w, labels, sel):
        rect(x, 4.6, w, 9.2, 0.6, "url(#g909hdr)", 'stroke="#1e1e1e" stroke-width="0.3"')
        rect(x + 0.4, 4.9, w - 0.8, 0.5, 0, "#fff", 'opacity="0.25"')
        for (lx, lab, idx) in labels:
            lit = idx == sel
            if lit:
                text(lx, 12.3, lab, 8.2, "#ff9a3c", "middle", "bold", 0.5, 'filter="url(#b1)" opacity="0.9"')
            text(lx, 12.3, lab, 8.2, "#ffb061" if lit else "#f0732a", "middle", "bold", 0.5)
    chip(6.4, 18.4, [(15.6, "AC", 0)], selected)
    for k in range(5):
        chip(26.8 + 42 * k, 39.6, [(46.6 + 42 * k, names[k + 1], k + 1)], selected)
    for k in range(3):
        x = 236.8 + 42 * k
        chip(x, 39.6, [(x + 10.6, names[6 + 2 * k], 6 + 2 * k), (x + 29.4, names[7 + 2 * k], 7 + 2 * k)], selected)
    for x in [4.6, 25.8] + [67.8 + 42 * k for k in range(8)]:
        y1 = 62 if x > 5 else 62
        line(x, 15.5, x, 67 if x > 26 else 61, "#777772", 0.5); line(x + 0.5, 15.5, x + 0.5, 67 if x > 26 else 61, "#fff", 0.4, 'opacity="0.9"')
    def lab(cx, y, s): text(cx, y, s, 5.2, ink, "middle", "bold", 0.1)
    knob_909(9, 28, 0.5)
    r1 = {30: "TUNE", 50: "LEVEL", 72: "TUNE", 92: "LEVEL", 114: "TUNE", 134: "LEVEL", 156: "TUNE", 176: "LEVEL",
          198: "TUNE", 218: "LEVEL", 240: "LEVEL", 260: "LEVEL", 302: "LEVEL", 324: "LEVEL", 344: "LEVEL"}
    r2 = {30: "ATT", 50: "DEC", 72: "TONE", 92: "SNAP", 114: "DEC", 156: "DEC", 198: "DEC", 282: "DEC", 302: "DEC",
          324: "TUNE", 344: "TUNE"}
    vals = [0.5, 0.74, 0.5, 0.66, 0.5, 0.66, 0.5, 0.66, 0.5, 0.66, 0.66, 0.66, 0.66, 0.66, 0.66]
    for (x, name), v in zip(r1.items(), vals):
        lab(x + 6, 24.2, name); knob_909(x, 28, v)
    for i, (x, name) in enumerate(r2.items()):
        lab(x + 6, 50.4, name); knob_909(x, 54, 0.5 if name != "DEC" or x > 250 else 0.86)
    # AC select
    rect(6.2, 49.6, 17.6, 8.4, 1.0, "none", 'stroke="#55554f" stroke-width="0.5"')
    led_round(10, 53.8, 1.1, "R", selected == 0)
    text(17, 55.7, "AC", 5.0, ink, "middle", "bold", 0.1)
    # flam
    rect(4.6, 62.6, 20.6, 50.2, 1.0, "none", 'stroke="#77776f" stroke-width="0.5"')
    lab(15, 70.6, "FLAM")
    knob_909(9, 76, 0.75)
    if flam:
        circle(15, 100, 11, "url(#haloR)")
    rect(7.6, 92.6, 14.8, 14.8, 1.2, "#3b3b39")
    rect(8.6, 93.6, 12.8, 12.8, 0.8, "url(#gCapLit)" if flam else "url(#gSilver)")
    if flam:
        rect(11.4, 96.4, 7.2, 7.2, 1.0, "#ff3a2a", 'opacity="0.8" filter="url(#b05)"')
        rect(12.2, 97.2, 5.6, 5.6, 0.6, "url(#ledR)", 'stroke="#b3140c" stroke-width="0.3"')
        rect(12.9, 97.8, 4.2, 1.6, 0.4, "#fff", 'opacity="0.55"')
    else:
        rect(12.2, 97.2, 5.6, 5.6, 0.6, "#b9bab6", 'stroke="#8a8b87" stroke-width="0.3"')
    # name labels
    full = ["BASS DRUM", "SNARE DRUM", "LOW TOM", "MID TOM", "HI TOM"]
    for k in range(5):
        x = 28.4 + 42 * k
        rect(x, 74.4, 36.4, 7.8, 1.2, "#f6f5f1", 'stroke="#4c4c47" stroke-width="0.5"')
        led_round(x + 3.6, 78.3, 1.05, "R", selected == k + 1)
        text(x + 20.6, 79.8, full[k], 3.9, ink, "middle", "bold", 0)
    for k in range(3):
        x = 238.4 + 42 * k
        rect(x, 74.4, 36.4, 7.8, 1.2, "#f6f5f1", 'stroke="#4c4c47" stroke-width="0.5"')
        led_round(x + 3.6, 78.3, 1.05, "R", selected == 6 + 2 * k)
        led_round(x + 32.8, 78.3, 1.05, "R", selected == 7 + 2 * k)
        text(x + 11.6, 80.0, names[6 + 2 * k], 4.6, ink, "middle", "bold", 0.1)
        text(x + 24.8, 80.0, names[7 + 2 * k], 4.6, ink, "middle", "bold", 0.1)
    # step keys
    rect(26.6, 103.6, 337, 7.6, 0.6, "url(#g909hdr)")
    for i in range(16):
        x = 29 + 21 * i
        rect(x + 0.9, 86.6, 16.2, 16.6, 1.8, "#000", 'opacity="0.5" filter="url(#b05)"')
        rect(x, 85.4, 16.4, 16.8, 1.7, "#2c2c2a")
        rect(x + 1.0, 86.4, 14.4, 14.8, 1.2, "url(#gSilver)")
        rect(x + 2.2, 90.6, 12.0, 9.0, 1.4, "#fff", 'opacity="0.55"')
        v = steps.get(i, 0)
        col = {0: None, 1: ("#ff9a3a", "O"), 2: ("#ff3b2c", "R"), 3: ("#49e03c", "G")}[v]
        if col:
            a(f'<ellipse cx="{n(x+8.2)}" cy="{n(89.2)}" rx="7" ry="4" fill="url(#halo{col[1]})"/>')
        rect(x + 4.6, 87.8, 7.2, 2.8, 0.7, col[0] if col else "#6a1d16", 'stroke="#2a0a07" stroke-width="0.3"')
        text(x + 8.2, 109.25, str(i + 1), 5.2, "#f1f1ee", "middle", "bold", 0)
    eg()

def mixer(c, on, pan, send, level, dist, pcf, comp, meter):
    g(f"translate(456 {50 + 115 * c})")
    plate_edges(0, 0, 70, 115)
    header(3, 4, 64, 14, "gHdrMix", "MIX", 35, 9.2, 1.2)
    onoff(3, 4, 17, 14, on, "#4f4a1a")
    meter4(51, 4, 14, 14, meter)
    knob_gray(10, 25, pan)
    knob_scale(10, 25, "L", "R", "PAN")
    fader(10, 59, level)
    for i, (name, state) in enumerate((("DIST", dist), ("PCF", pcf), ("COMP", comp))):
        route_switch(42, 19 + 18 * i, state)
        stext(44.3, 36.0 + 18 * i, name, 6.2, anchor="start", ls=0.5 if name == "PCF" else 0.2)
    knob_gray(42, 79, send)
    knob_scale(42, 79, "0", "10", "DELAY")
    eg()

def master(level, l, r, comp):
    g("translate(526 50)")
    plate_edges(0, 0, 82, 98)
    header(4, 4, 74, 14, "gHdrMix", "MASTER", 41, 9.0, 0.6)
    for (x, lv, side) in ((16, l, -1), (51, r, 1)):
        rect(x + 4.2, 26.4, 6.6, 51.2, 0.8, "#0d0f10")
        for i in range(10):
            on = (9 - i) < lv
            col = ("#ff3b2c", "#4a1310") if i < 2 else (("#ffc42e", "#4a3a10") if i < 4 else ("#43e03a", "#143f1a"))
            yy = 27.6 + 5 * i
            if on:
                rect(x + 3.6, yy - 0.7, 7.8, 4.4, 1, col[0], 'opacity="0.45" filter="url(#b05)"')
            rect(x + 5.2, yy, 4.6, 2.8, 0.4, col[0] if on else col[1])
    for i, s in enumerate(("Clip", "-6", "-12", "-24", "-36")):
        yy = 31.4 + 10 * i
        stext(14.4, yy, s, 5.6, anchor="end", weight="500", ls=0)
        stext(67.6, yy, s, 5.6, anchor="start", weight="500", ls=0)
    fader(33, 25, level)
    stext(39.6, 88.7, "COMP", 6.2, anchor="end")
    route_switch(42, 81, comp)
    eg()

def fx_header(title, on, meter, y=2.5, ls=0.4):
    header(3, y, 76, 15, "gHdrFx", title, 41, 9.2, ls)
    onoff(3, y - 0.5, 16, 16, on, "#14244f")
    meter4(63, y + 0.5, 14, 14, meter)

def pcf(on, pattern, mode, f):
    g("translate(526 148)")
    plate_edges(0, 0, 82, 111)
    fx_header("PCF", on, 0, 2.5, 3.2)
    display(10, 23, 18, 16, pattern); arrows(29, 22)
    slide_v(49, 21, 9, 18, mode)
    stext(61, 28.2, "BP", 5.8, anchor="start"); stext(61, 37.6, "LP", 5.8, anchor="start")
    stext(24.5, 46.8, "PATTERN", 6.0); stext(63, 46.8, "MODE", 6.0)
    for i, name in enumerate(("FREQ", "Q", "AMT", "DEC")):
        fader(4 + 19 * i, 50, f[i])
    for i, name in enumerate(("FREQ", "Q", "AMT", "DEC")):
        pass
    eg()

def pcf_labels():
    g("translate(526 148)")
    for i, name in enumerate(("FREQ", "Q", "AMT", "DEC")):
        stext(11.5 + 19 * i, 108.4, name, 5.8, ls=0.1)
    eg()

def delay(on, steps, mode, pan, fb):
    g("translate(526 259)")
    plate_edges(0, 0, 82, 93)
    fx_header("DELAY", on, 4, 3)
    display(9, 23, 19, 16, steps); arrows(29, 22)
    slide_v(49, 22, 9, 18, mode)
    # triplet sign and a note
    a('<path d="M60.6 26.6 Q61.4 23.6 64.2 23.4 M68.6 23.4 Q71.4 23.6 72.2 26.6" fill="none" stroke="#edf1f3" stroke-width="0.6" stroke-linecap="round"/>')
    stext(66.4, 26.4, "3", 5.6, ls=0)
    g("translate(66 39.2) scale(0.62)")
    a('<ellipse cx="0" cy="0" rx="2.5" ry="1.8" transform="rotate(-22)" fill="#edf1f3"/><path d="M2.1 -0.6 V-11.6" stroke="#edf1f3" stroke-width="0.9" fill="none"/>'
      '<path d="M2.1 -11.6 C3 -8.6 6.2 -8.2 5.2 -4.6 C5.4 -6.8 3.4 -7.6 2.1 -8.2 Z M2.1 -8.4 C3 -5.6 6 -5.2 5 -1.8 C5.2 -3.8 3.4 -4.6 2.1 -5.2 Z" fill="#edf1f3"/>')
    eg()
    stext(18.5, 46.6, "STEPS", 6.0)
    knob_gray(12, 58, pan); knob_gray(53, 58, fb)
    knob_scale(12, 58, "L", "R", "PAN"); knob_scale(53, 58, "0", "10", "F.BACK")
    eg()

def dist(on, amount, shape):
    g("translate(526 352)")
    plate_edges(0, 0, 82, 65)
    fx_header("DIST", on, 1, 3, 0.6)
    knob_gray(12, 28, amount); knob_gray(53, 28, shape)
    knob_scale(12, 28, "0", "10", "AMOUNT"); knob_scale(53, 28, "0", "10", "SHAPE")
    eg()

def comp(on, ratio, thres, gr):
    g("translate(526 417)")
    plate_edges(0, 0, 82, 93)
    fx_header("COMP", on, 3, 3, 0.8)
    for i in range(9):
        col = "G" if i < 3 else ("Y" if i < 6 else "R")
        led_round(11.5 + 7 * i, 30, 1.9, col, i in gr)
    for i, s in enumerate(("-40", "-20", "0", "+20", "+40")):
        stext(11.5 + 14 * i, 41.4, s, 5.2, ls=0, weight="500")
    knob_gray(12, 57, ratio); knob_gray(53, 57, thres)
    knob_scale(12, 57, "0", "10", "RATIO"); knob_scale(53, 57, "0", "10", "THRES")
    eg()

def backdrop(W, dx):
    """What shows left and right of the unit in a wide window: a dark matte desk with a faint
    horizontal brushing, darker toward the window's edges, and the shadow the unit casts on it.
    All of it is a gradient or procedural, so it fits any width."""
    a('<defs>')
    lg("gDesk", [(0, "#1d2023"), (1, "#131517")])
    rg("gVignette", [(0, "#000", 0), (0.62, "#000", 0), (1, "#000", 0.5)], 0.5, 0.5, 0.75)
    a('<filter id="brush" x="0" y="0" width="1" height="1" color-interpolation-filters="sRGB">'
      '<feTurbulence type="fractalNoise" baseFrequency="0.006 0.55" numOctaves="3" seed="12"/>'
      '<feColorMatrix type="matrix" values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  2.2 0 0 0 -0.95"/>'
      '<feComposite in2="SourceGraphic" operator="in"/></filter>')
    a('<filter id="bCast" x="-400%" y="-5%" width="900%" height="110%"><feGaussianBlur stdDeviation="5"/></filter>')
    a('</defs>')
    rect(0, 0, W, 510, 0, "url(#gDesk)")
    rect(0, 0, W, 510, 0, "#000", 'filter="url(#brush)" opacity="0.10"')
    rect(0, 0, W, 510, 0, "#000", 'filter="url(#grD)" opacity="0.5"')
    rect(0, 0, W, 510, 0, "#000", 'filter="url(#grL)" opacity="0.07"')
    rect(0, 0, W, 510, 0, "url(#gVignette)")
    # the unit's shadow: light from the upper left, as on the knobs
    rect(dx - 5, 0, 12, 510, 0, "#000", 'filter="url(#bCast)" opacity="0.55"')
    rect(dx + 616, 0, 20, 510, 0, "#000", 'filter="url(#bCast)" opacity="0.85"')

def main(out, splash, mod_name, W=624):
    dx = (W - 624) / 2
    a(f'<svg xmlns="http://www.w3.org/2000/svg" width="{n(W)}" height="510" viewBox="0 0 {n(W)} 510">')
    defs()
    if W > 624:
        backdrop(W, dx)
    g(f"translate({n(dx)} 0)")
    rect(0, 0, 624, 510, 0, "#14181b")
    gray_base()
    grain_over(gray_rects(), "cGray", (("grD", 0.46), ("grL", 0.32), ("blotD", 0.11), ("blotL", 0.06)))
    wood(0); wood(608)
    transport()
    for r, (p, b) in enumerate(((0, 0), (0, 0), (0, 0), (1, 0))):
        pattern_section(r, p, b, on=(r != 1), focus=(r == 0), shuffle=(r == 1))
    tb303(0, (0.42, 0.34, 0.5, 0.32, 0.5, 0.5), 1, 1, 0, ('rest',), pitch=True)
    tb303(1, (0.5, 0.28, 0.62, 0.3, 0.72, 0.4), 0, 1, 0, ('note', 'accent'), pressed=('back', 'step'))
    tr808({0, 4, 8, 12}, 1, splash, mod_name)
    tr909({0: 2, 4: 2, 8: 2, 12: 2, 14: 1, 10: 3}, 1, flam=True)
    mixer(0, True, 0.5, 0.2, 0.72, False, False, False, 2)
    mixer(1, True, 0.5, 0.2, 0.72, True, False, False, 2)
    mixer(2, True, 0.5, 0.2, 0.72, False, False, False, 3)
    mixer(3, True, 0.5, 0.2, 0.72, False, False, False, 1)
    master(0.8, 6, 6, True)
    pcf(True, 25, 1, (0.0, 0.55, 1.0, 0.3)); pcf_labels()
    delay(True, 3, 1, 0.5, 0.42)
    dist(True, 0.5, 0.36)
    comp(True, 0.52, 0.5, {0, 1, 2, 3, 4})
    eg()
    a('</svg>')
    open(out, 'w').write('\n'.join(O))

if __name__ == '__main__':
    main(sys.argv[1], sys.argv[2], "Standard ReBirth", float(sys.argv[3]) if len(sys.argv) > 3 else 624)
