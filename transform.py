import re

# 1. Strip the docs section from index.html
with open('index.html', 'r') as f:
    html = f.read()

# find <section class="docs" id="docs"> ... </section>
# It ends right before <section class="playground" id="playground">
html = re.sub(r'<div class="motif-line">.*?</div>\s*<section class="docs" id="docs">.*?</section>', '', html, flags=re.DOTALL)
with open('index.html', 'w') as f:
    f.write(html)

# 2. Helper to generate a new page based on existing content but with new layout
def create_page(filename, title, kicker, hero_title, hero_subtitle, sections_html, sidebar_links):
    sidebar = '<aside class="docs-sidebar"><span class="sidebar-kicker">On this page</span>' + ''.join(f'<a href="{url}">{text}</a>' for text, url in sidebar_links) + '</aside>'
    
    content = f"""<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title} — Vinglish</title>
  <link rel="icon" type="image/svg+xml" href="logos/vinglish-favicon-16.svg">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=Baloo+2:wght@500;600;700;800&family=Shrikhand&family=Space+Mono:wght@400;700&display=swap" rel="stylesheet">
  <link rel="stylesheet" href="site.css">
  <style>
    /* Adjust hero for inner pages */
    .hero-inner {{ min-height: auto; padding: 72px 0; text-align: center; display: block; }}
    .hero-inner h1 {{ margin: 18px auto; max-width: 800px; }}
    .hero-inner p {{ margin: 0 auto; max-width: 650px; }}
  </style>
</head>
<body>
  <div class="top-strip"><div class="marquee">✦ {kicker} ✦ {kicker} ✦ {kicker} ✦</div></div>
  <header class="topbar">
    <div class="shell nav">
      <a class="brand" href="index.html"><img src="logos/vinglish-icon-color.svg" alt=""><span>Vinglish</span></a>
      <nav class="nav-links">
        <a href="index.html">Home</a>
        <a href="language_guide.html" {'aria-current="page"' if filename=='language_guide.html' else ''}>Language Guide</a>
        <a href="architecture.html" {'aria-current="page"' if filename=='architecture.html' else ''}>Architecture</a>
        <a href="standard_library.html" {'aria-current="page"' if filename=='standard_library.html' else ''}>Standard Library</a>
      </nav>
      <a class="nav-cta" href="index.html#get-started">Begin building →</a>
    </div>
  </header>
  <main>
    <section class="shell hero hero-inner">
      <div class="reveal">
        <span class="eyebrow">AUTHORITATIVE REFERENCE</span>
        <h1>{hero_title}<span>{hero_subtitle}</span></h1>
      </div>
    </section>
    
    <div class="motif-line">✦ ☀ ✦ VINGLISH · {title.upper()} ✦ ☀ ✦</div>
    
    <section class="docs">
      <div class="shell">
        <div class="docs-layout">
          {sidebar}
          <article class="doc-page">
            {sections_html}
          </article>
        </div>
      </div>
    </section>
  </main>
  <footer>
    <div class="shell footer-inner">
      <span>VINGLISH · READABLE SYSTEMS PROGRAMMING</span>
      <nav class="footer-links">
        <a href="language_guide.html">Guide</a>
        <a href="architecture.html">Architecture</a>
        <a href="standard_library.html">Library</a>
      </nav>
    </div>
  </footer>
  <script>
    const sections = document.querySelectorAll(".doc-section");
    const sideLinks = document.querySelectorAll(".docs-sidebar a");
    const sectionObserver = new IntersectionObserver((entries) => entries.forEach((entry) => {{
      if (!entry.isIntersecting) return;
      sideLinks.forEach((link) => link.classList.toggle("active", link.getAttribute("href") === "#" + entry.target.id));
    }}), {{ rootMargin: "-20% 0px -65% 0px", threshold: 0 }});
    sections.forEach((section) => sectionObserver.observe(section));
  </script>
</body>
</html>"""
    with open(filename, 'w') as f:
        f.write(content)

# Process language_guide.html
with open('language_guide.html', 'r') as f: html = f.read()
sections = re.findall(r'<section class="section" id="(.*?)"><span class="tag">(.*?)</span><h2>(.*?)</h2>(.*?)</section>', html, re.DOTALL)
sections_html = ""
sidebar_links = []
for (sid, stag, sh2, sbody) in sections:
    sidebar_links.append((sh2, f"#{sid}"))
    sbody = sbody.replace('class="grid"', 'class="doc-card-grid"').replace('class="card"', 'class="doc-card"').replace('<b>', '<span>').replace('</b>', '</span>')
    sections_html += f'<section class="doc-section" id="{sid}"><p class="section-tag">{stag}</p><h2>{sh2}</h2>{sbody}</section>\n'
create_page('language_guide.html', 'Language Guide', 'VINGLISH · LANGUAGE GUIDE · READABLE SYSTEMS PROGRAMMING', 'Write what you mean.<br>', 'Compile what you intend.', sections_html, sidebar_links)

# Process architecture.html
with open('architecture.html', 'r') as f: html = f.read()
sections = re.findall(r'<section class="section" id="(.*?)"><span class="tag">(.*?)</span><h2>(.*?)</h2>(.*?)</section>', html, re.DOTALL)
sections_html = ""
sidebar_links = []
for (sid, stag, sh2, sbody) in sections:
    sidebar_links.append((sh2, f"#{sid}"))
    sbody = sbody.replace('class="grid"', 'class="doc-card-grid"').replace('class="card"', 'class="doc-card"').replace('<b>', '<span>').replace('</b>', '</span>')
    sections_html += f'<section class="doc-section" id="{sid}"><p class="section-tag">{stag}</p><h2>{sh2}</h2>{sbody}</section>\n'
create_page('architecture.html', 'Architecture', 'VINGLISH · COMPILER ARCHITECTURE · MIR ALL THE WAY DOWN', 'Readable source.<br>', 'Explicit machinery.', sections_html, sidebar_links)

# Process standard_library.html
with open('standard_library.html', 'r') as f: html = f.read()
sections = re.findall(r'<section class="section" id="(.*?)"><span class="tag">(.*?)</span><h2>(.*?)</h2>(.*?)</section>', html, re.DOTALL)
sections_html = ""
sidebar_links = []
for (sid, stag, sh2, sbody) in sections:
    sidebar_links.append((sh2, f"#{sid}"))
    sbody = sbody.replace('class="grid"', 'class="doc-card-grid"').replace('class="card"', 'class="doc-card"').replace('<b>', '<span>').replace('</b>', '</span>')
    sections_html += f'<section class="doc-section" id="{sid}"><p class="section-tag">{stag}</p><h2>{sh2}</h2>{sbody}</section>\n'
create_page('standard_library.html', 'Standard Library', 'VINGLISH · STANDARD LIBRARY · NATIVE CAPABILITIES', 'Native capabilities.<br>', 'Zero overhead.', sections_html, sidebar_links)

print("Pages transformed.")
