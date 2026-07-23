import re

with open('index.html', 'r') as f:
    html = f.read()

# Extract the style block
style_match = re.search(r'<style>(.*?)</style>', html, re.DOTALL)
if style_match:
    css = style_match.group(1).strip()
    with open('site.css', 'w') as f:
        f.write(css)

# Replace style block with link
new_html = re.sub(r'<style>.*?</style>', '<link rel="stylesheet" href="site.css">', html, flags=re.DOTALL)
with open('index.html', 'w') as f:
    f.write(new_html)

print("Extracted CSS and updated index.html")
