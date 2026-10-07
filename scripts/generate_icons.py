import os
from PIL import Image, ImageDraw

src_path = 'icon.png'
if not os.path.exists(src_path):
    print("Error: icon.png not found")
    exit(1)

src = Image.open(src_path).convert('RGBA')

res_dir = os.path.join('src-tauri', 'gen', 'android', 'app', 'src', 'main', 'res')
bg_color = (9, 4, 18, 255) # Deep luxury purple/black #090412

densities = {
    'mipmap-mdpi': {'canvas': 108, 'fg_icon': 74, 'legacy': 48},
    'mipmap-hdpi': {'canvas': 162, 'fg_icon': 111, 'legacy': 72},
    'mipmap-xhdpi': {'canvas': 216, 'fg_icon': 148, 'legacy': 96},
    'mipmap-xxhdpi': {'canvas': 324, 'fg_icon': 222, 'legacy': 144},
    'mipmap-xxxhdpi': {'canvas': 432, 'fg_icon': 296, 'legacy': 192},
}

for folder, dims in densities.items():
    folder_path = os.path.join(res_dir, folder)
    os.makedirs(folder_path, exist_ok=True)
    
    # 1. Adaptive foreground (108dp canvas, icon scaled to safe zone, transparent background)
    c_size = dims['canvas']
    icon_size = dims['fg_icon']
    fg_canvas = Image.new('RGBA', (c_size, c_size), (0, 0, 0, 0))
    scaled_icon = src.resize((icon_size, icon_size), Image.Resampling.LANCZOS)
    offset = (c_size - icon_size) // 2
    fg_canvas.paste(scaled_icon, (offset, offset), scaled_icon)
    fg_canvas.save(os.path.join(folder_path, 'ic_launcher_foreground.png'), 'PNG')
    
    # 2. Legacy square icon
    l_size = dims['legacy']
    legacy_canvas = Image.new('RGBA', (l_size, l_size), bg_color)
    l_scaled = src.resize((l_size, l_size), Image.Resampling.LANCZOS)
    legacy_canvas.paste(l_scaled, (0, 0), l_scaled)
    mask = Image.new('L', (l_size, l_size), 0)
    draw = ImageDraw.Draw(mask)
    radius = int(l_size * 0.22)
    draw.rounded_rectangle([0, 0, l_size - 1, l_size - 1], radius=radius, fill=255)
    legacy_final = Image.new('RGBA', (l_size, l_size), (0, 0, 0, 0))
    legacy_final.paste(legacy_canvas, (0, 0), mask)
    legacy_final.save(os.path.join(folder_path, 'ic_launcher.png'), 'PNG')
    
    # 3. Legacy round icon
    round_mask = Image.new('L', (l_size, l_size), 0)
    draw_round = ImageDraw.Draw(round_mask)
    draw_round.ellipse([0, 0, l_size - 1, l_size - 1], fill=255)
    round_final = Image.new('RGBA', (l_size, l_size), (0, 0, 0, 0))
    round_final.paste(legacy_canvas, (0, 0), round_mask)
    round_final.save(os.path.join(folder_path, 'ic_launcher_round.png'), 'PNG')
    
    print(f'Generated icons for {folder}: canvas {c_size}x{c_size}, legacy {l_size}x{l_size}')

# Create mipmap-anydpi-v26 XMLs
anydpi_dir = os.path.join(res_dir, 'mipmap-anydpi-v26')
os.makedirs(anydpi_dir, exist_ok=True)

adaptive_xml = '''<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@drawable/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
'''

with open(os.path.join(anydpi_dir, 'ic_launcher.xml'), 'w', encoding='utf-8') as f:
    f.write(adaptive_xml)

with open(os.path.join(anydpi_dir, 'ic_launcher_round.xml'), 'w', encoding='utf-8') as f:
    f.write(adaptive_xml)

print('Android adaptive and legacy icons generated successfully.')
