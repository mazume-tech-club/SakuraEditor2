"""Sakura2 のアイコン（エディタの紙 + 桜の花）を生成する。
使い方: python tools/make_icon.py  → assets/sakura2.ico と assets/sakura2.png
"""
import math
from PIL import Image, ImageDraw, ImageFilter

S = 1024  # 作画サイズ（縮小して各サイズを作る）
PINK = (245, 140, 175, 255)
PINK_DARK = (214, 86, 132, 255)
PINK_LIGHT = (255, 205, 222, 255)
CENTER = (255, 236, 150, 255)
PAGE = (255, 255, 255, 255)
PAGE_EDGE = (120, 130, 150, 255)
LINE = (175, 185, 205, 255)
BG = (60, 90, 150, 255)


def petal(cx, cy, r, angle, notch=True):
    """中心 (cx,cy) から angle 方向へ伸びる、先端に切れ込みのある花びら"""
    pts = []
    n = 40
    for i in range(n + 1):
        t = i / n
        w = r * 0.36 * math.sin(math.pi * min(t, 0.999)) ** 0.7
        pts.append((w, t * r))
    right = pts
    if notch:
        tip = [(r * 0.10, r * 0.97), (0, r * 0.84), (-r * 0.10, r * 0.97)]
    else:
        tip = [(0, r)]
    left = [(-x, y) for x, y in reversed(right)]
    shape = right[:-4] + tip + left[4:]
    a = math.radians(angle)
    ca, sa = math.cos(a), math.sin(a)
    return [(cx + x * ca - y * sa, cy + x * sa + y * ca) for x, y in shape]


def flower(d, cx, cy, r, rot=0, outline=True):
    for k in range(5):
        ang = rot + k * 72
        d.polygon(petal(cx, cy, r, ang), fill=PINK, outline=PINK_DARK if outline else None, width=max(1, int(r * 0.04)))
        # 花びらの中心線（淡い）
        inner = petal(cx, cy, r * 0.55, ang, notch=False)
        d.polygon(inner, fill=PINK_LIGHT)
    cr = r * 0.17
    d.ellipse([cx - cr, cy - cr, cx + cr, cy + cr], fill=CENTER, outline=PINK_DARK, width=max(1, int(r * 0.03)))
    # しべ
    for k in range(5):
        a = math.radians(rot + 36 + k * 72 + 90)
        x, y = cx + math.cos(a) * r * 0.30, cy + math.sin(a) * r * 0.30
        d.line([cx, cy, x, y], fill=PINK_DARK, width=max(1, int(r * 0.025)))
        dr = r * 0.045
        d.ellipse([x - dr, y - dr, x + dr, y + dr], fill=PINK_DARK)


def render(small=False):
    img = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    # 紙（右上が折れたドキュメント）
    m = 110 if not small else 60
    x0, y0, x1, y1 = m, 60, S - m - 40, S - 70
    fold = 190
    page = [(x0, y0), (x1 - fold, y0), (x1, y0 + fold), (x1, y1), (x0, y1)]
    shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).polygon([(x + 18, y + 22) for x, y in page], fill=(0, 0, 0, 90))
    img.alpha_composite(shadow.filter(ImageFilter.GaussianBlur(18)))
    d.polygon(page, fill=PAGE, outline=PAGE_EDGE, width=14)
    d.polygon([(x1 - fold, y0), (x1 - fold, y0 + fold), (x1, y0 + fold)], fill=(225, 230, 240, 255), outline=PAGE_EDGE, width=12)
    if not small:
        # テキスト行（コードっぽく長さを変える）
        widths = [0.55, 0.75, 0.40, 0.65, 0.30]
        for i, wv in enumerate(widths):
            y = y0 + 300 + i * 95
            if y > y1 - 330 and i > 1:
                break
            d.rounded_rectangle([x0 + 70, y, x0 + 70 + (x1 - x0 - 140) * wv, y + 40], radius=20, fill=LINE)
    # 桜（右下に大きく）
    if small:
        flower(d, S * 0.52, S * 0.56, S * 0.40, rot=180)
    else:
        flower(d, S * 0.64, S * 0.66, S * 0.33, rot=180)
    return img


def main():
    big = render(False)
    small = render(True)
    sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256]
    frames = []
    for s in sizes:
        src = small if s <= 24 else big
        frames.append(src.resize((s, s), Image.LANCZOS))
    big.resize((256, 256), Image.LANCZOS).save("assets/sakura2.png")
    frames[-1].save("assets/sakura2.ico", sizes=[(s, s) for s in sizes], append_images=frames[:-1])
    # 確認用の一覧
    sheet = Image.new("RGBA", (sum(sizes) + 10 * len(sizes) + 10, 270), (240, 240, 240, 255))
    x = 10
    for s, f in zip(sizes, frames):
        sheet.alpha_composite(f, (x, 260 - s))
        x += s + 10
    sheet.save("assets/preview.png")


if __name__ == "__main__":
    main()
