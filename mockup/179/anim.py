#!/usr/bin/env python3
"""Preview of the wordmark's startup animation (crates/orb-ui/src/logo_anim.rs) with the redesign
art, in the main window and in Help > About: layers rendered with resvg and played in an HTML
page, plus film strips of key moments.

    anim.py RENDER SPLASH.jpg OUT_DIR

RENDER is the built renderer of ./render, SPLASH.jpg the standard mod's splash (README.md has
both commands). Writes OUT_DIR/logo_animation.html, logo_animation_main_frames.png and
logo_animation_about_frames.png; the working files go to OUT_DIR/work."""
import base64, os, subprocess, sys
import mock
from mock import a, n, O

HERE = os.path.dirname(os.path.abspath(__file__))
RENDER, SPLASH, DEST = (os.path.abspath(p) for p in sys.argv[1:4])
OUT = os.path.join(DEST, "work")
FONT_DIR = os.path.join(HERE, "fonts")
ICON = os.path.join(HERE, "..", "..", "assets", "ui", "app_icon.svg")
os.makedirs(OUT, exist_ok=True)

def write(name):
    path = os.path.join(OUT, name)
    open(path, "w").write("\n".join(O))
    O.clear()
    return path

def render(svg, png, scale):
    subprocess.check_call([RENDER, svg, png, str(scale), FONT_DIR], stdout=subprocess.DEVNULL)
    return png

def b64(path, mime):
    return f"data:{mime};base64," + base64.b64encode(open(path, "rb").read()).decode()

# ---- 1. the panel without its logo
mock.DRAW_LOGO = False
O.clear()
mock.main(os.path.join(OUT, "panel_nologo.svg"), SPLASH, "Standard ReBirth")
O.clear()
mock.DRAW_LOGO = True
panel2 = render(os.path.join(OUT, "panel_nologo.svg"), os.path.join(OUT, "panel_nologo_2x.png"), 2)
panel3 = render(os.path.join(OUT, "panel_nologo.svg"), os.path.join(OUT, "panel_nologo_3x.png"), 3)
panel_jpg = os.path.join(OUT, "panel_nologo_2x.jpg")
subprocess.check_call(["sips", "-s", "format", "jpeg", "-s", "formatOptions", "92", panel2, "--out", panel_jpg],
                      stdout=subprocess.DEVNULL)

# ---- 2. the logo pieces, each on the whole 134 x 48 canvas
pieces = {}
for name in mock.LOGO_PIECES:
    a('<svg xmlns="http://www.w3.org/2000/svg" width="134" height="48" viewBox="0 0 134 48">')
    mock.defs()
    mock.logo_piece(name)
    a('</svg>')
    svg = write(f"logo_{name}.svg")
    pieces[name] = render(svg, os.path.join(OUT, f"logo_{name}.png"), 6)

# ---- 3. the About window's content: a plate like the transport's, 450 x 200 (about.rs)
VERSION = "Version 2026.1113.1200 (dev-8398062)"
URL = "https://github.com/pingw33n/OpenRB-338"
COLLECTIONS = (("Song collection:", "https://github.com/pingw33n/rb-songs"),
               ("Mod collection:", "https://github.com/pingw33n/rb-mods"))
WINDOW_BG = "#1b1b1b"       # egui's dark window fill, as the app's About window has now
INK_DIM = "#9aa7b8"         # palette.rs
LINK = "#5aaaff"            # egui's hyperlink color
PLATE = (92, 14, 268, 92)   # the gray plate under the wordmark (the logo canvas at 2x, less 4 below)
ICON_Y = 28                 # centered on the plate (the app has 18)
ROW0 = 123                  # the first text baseline (6 lower than the app's rows, to clear the plate)

def about_plate():
    """The About content: the window's own dark background, and only the wordmark on a gray plate."""
    mock.rect(0, 0, 450, 200, 0, WINDOW_BG)
    x, y, w, h = PLATE
    mock.rect(x + 1.5, y + 2.5, w, h, 2, "#000", 'opacity="0.6" filter="url(#b2)"')
    mock.plate(x, y, w, h, mock.GRAY)
    mock.grain_over([PLATE], "cAbout", (("grD", 0.46), ("grL", 0.32), ("blotD", 0.11), ("blotL", 0.06)))
    mock.plate_edges(x, y, w, h)
    t = lambda tx, ty, s, col: mock.text(tx, ty, s, 11.6, col, "start", "normal", 0)
    t(108, ROW0, VERSION, INK_DIM)
    t(108, ROW0 + 22, URL, LINK)
    for i, (label, url) in enumerate(COLLECTIONS):
        t(108, ROW0 + 44 + 22 * i, label, INK_DIM)
        t(208, ROW0 + 44 + 22 * i, url, LINK)

a('<svg xmlns="http://www.w3.org/2000/svg" width="450" height="200" viewBox="0 0 450 200">')
mock.defs(); about_plate(); a('</svg>')
about_svg = write("about_plate.svg")
about_png = render(about_svg, os.path.join(OUT, "about_plate.png"), 2)
about3 = render(about_svg, os.path.join(OUT, "about_plate_3x.png"), 3)
icon_png = render(ICON, os.path.join(OUT, "app_icon.png"), 4)

# ---- 4. film strips
TIMES = (0.0, 0.15, 0.35, 0.6, 0.9, 1.2, 1.5, 1.85)

def strip_main():
    cw, ch, z = 250, 52, 3        # the right end of the transport, in panel units
    W, H = cw * z + 150, (ch * z + 10) * len(TIMES) + 10
    a(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">')
    mock.defs()
    mock.rect(0, 0, W, H, 0, "#101214")
    for i, t in enumerate(TIMES):
        y = 10 + (ch * z + 10) * i
        mock.text(16, y + ch * z / 2 + 8, f"{t:.2f} s", 26, "#d7dfe4", "start", "500")
        a(f'<svg x="150" y="{y}" width="{cw*z}" height="{ch*z}" viewBox="{624-cw} 0 {cw} {ch}">')
        a(f'<image x="0" y="0" width="624" height="510" href="{panel3}"/>')
        mock.logo(474, 1, None if t >= 1.85 else t)
        a('</svg>')
    a('</svg>')
    return render(write("strip_main.svg"), os.path.join(OUT, "strip_main.png"), 1)

def strip_about():
    z = 1.5
    cw, ch = 450 * z, 200 * z
    W, H = int(cw * 2 + 30), int((ch + 44) * 4 + 10)
    a(f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">')
    mock.defs()
    mock.rect(0, 0, W, H, 0, "#101214")
    for i, t in enumerate(TIMES):
        x = 10 + (cw + 10) * (i % 2); y = 10 + (ch + 44) * (i // 2)
        mock.text(x + 4, y + 24, f"{t:.2f} s", 24, "#d7dfe4", "start", "500")
        a(f'<svg x="{n(x)}" y="{n(y+34)}" width="{n(cw)}" height="{n(ch)}" viewBox="0 0 450 200">')
        a(f'<image x="0" y="0" width="450" height="200" href="{about3}"/>')
        ia = 1.0 if t >= 1.85 else mock.logo_frame(t)[3]
        a(f'<image x="18" y="{ICON_Y}" width="64" height="64" href="{icon_png}" opacity="{n(ia)}"/>')
        mock.logo(92, 14, None if t >= 1.85 else t, 2.0)
        a('</svg>')
    a('</svg>')
    return render(write("strip_about.svg"), os.path.join(OUT, "strip_about.png"), 1)

s_main = strip_main()
s_about = strip_about()

# ---- 5. the page
img = {k: b64(v, "image/png") for k, v in pieces.items()}
html = open(os.path.join(HERE, "anim_template.html")).read()
html = (html.replace("{{PANEL}}", b64(panel_jpg, "image/jpeg"))
            .replace("{{ABOUT}}", b64(about_png, "image/png"))
            .replace("{{ICON}}", b64(icon_png, "image/png")))
for k, v in img.items():
    html = html.replace("{{L_%s}}" % k, v)
dst = os.path.join(DEST, "logo_animation.html")
open(dst, "w").write(html)
for src, name in ((s_main, "logo_animation_main_frames.png"), (s_about, "logo_animation_about_frames.png")):
    subprocess.check_call(["cp", src, os.path.join(DEST, name)])
print(dst, os.path.getsize(dst))
