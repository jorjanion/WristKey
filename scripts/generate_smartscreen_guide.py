import os
from PIL import Image, ImageDraw, ImageFont

# Canvas dimensions
w, h = 900, 520
img = Image.new('RGBA', (w, h), (24, 27, 33, 255))
draw = ImageDraw.Draw(img)

# Outer dialog container (Windows 11 dark/accent modal)
modal_x, modal_y, modal_w, modal_h = 40, 30, 820, 460
draw.rounded_rectangle([(modal_x, modal_y), (modal_x + modal_w, modal_y + modal_h)], radius=16, fill=(18, 30, 49, 255), outline=(0, 120, 212, 180), width=2)

# Blue accent header banner
draw.rounded_rectangle([(modal_x + 2, modal_y + 2), (modal_x + modal_w - 2, modal_y + 80)], radius=14, fill=(0, 90, 158, 255))
draw.rectangle([(modal_x + 2, modal_y + 50), (modal_x + modal_w - 2, modal_y + 80)], fill=(0, 90, 158, 255))

try:
    font_title = ImageFont.truetype('segoeui.ttf', 24)
    font_sub = ImageFont.truetype('segoeui.ttf', 16)
    font_bold = ImageFont.truetype('segoeuib.ttf', 16)
    font_btn = ImageFont.truetype('segoeuib.ttf', 15)
    font_hint = ImageFont.truetype('segoeuib.ttf', 14)
except Exception:
    font_title = ImageFont.load_default()
    font_sub = font_title
    font_bold = font_title
    font_btn = font_title
    font_hint = font_title

# Title in header
draw.text((modal_x + 28, modal_y + 26), 'Windows protegio su PC / Windows protected your PC', fill=(255, 255, 255), font=font_title)

# Warning body text
body_text = (
    'SmartScreen de Microsoft Defender impidio el inicio de una aplicacion no reconocida.\n'
    'Microsoft Defender SmartScreen prevented an unrecognized app from starting.\n\n'
    'Esto ocurre porque WristKey es un proyecto open-source nuevo sin certificado EV de pago ($500/ano).\n'
    'This is 100% normal for open-source GitHub binaries without a paid $500/yr EV certificate.'
)
draw.text((modal_x + 32, modal_y + 98), body_text, fill=(220, 225, 235), font=font_sub)

# Step 1 Callout: 'Mas informacion'
step1_box = (modal_x + 32, modal_y + 205, modal_x + 380, modal_y + 245)
draw.rounded_rectangle(step1_box, radius=8, fill=(0, 120, 212, 60), outline=(0, 160, 255, 220), width=2)
draw.text((modal_x + 44, modal_y + 215), 'PASO 1: Haz clic en [ Mas informacion ]', fill=(100, 200, 255), font=font_bold)

# App details box
details_box = (modal_x + 32, modal_y + 265, modal_x + modal_w - 32, modal_y + 355)
draw.rounded_rectangle(details_box, radius=10, fill=(12, 18, 28, 255), outline=(40, 55, 75, 255), width=1)
draw.text((modal_x + 50, modal_y + 280), 'Aplicacion / App:   WristKey_x64-setup.exe', fill=(200, 210, 225), font=font_sub)
draw.text((modal_x + 50, modal_y + 312), 'Editor / Publisher:  Editor desconocido (Open Source Community Release)', fill=(160, 175, 195), font=font_sub)

# Step 2 Callout & Action Buttons
draw.text((modal_x + 32, modal_y + 375), 'PASO 2: Aparecera este boton. Haz clic en [ Ejecutar de todas formas ]:', fill=(74, 222, 128), font=font_hint)

# 'Ejecutar de todas formas' button
btn_run = (modal_x + 32, modal_y + 400, modal_x + 310, modal_y + 444)
draw.rounded_rectangle(btn_run, radius=8, fill=(16, 185, 129, 255))
draw.text((modal_x + 50, modal_y + 413), 'Ejecutar de todas formas', fill=(255, 255, 255), font=font_btn)

# 'No ejecutar' button
btn_cancel = (modal_x + 330, modal_y + 400, modal_x + 470, modal_y + 444)
draw.rounded_rectangle(btn_cancel, radius=8, fill=(45, 55, 72, 255))
draw.text((modal_x + 355, modal_y + 413), 'No ejecutar', fill=(200, 210, 225), font=font_btn)

os.makedirs('assets', exist_ok=True)
img.save('assets/smartscreen_guide.png', 'PNG')
print('assets/smartscreen_guide.png created successfully')
