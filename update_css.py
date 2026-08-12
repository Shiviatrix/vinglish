import re

with open('site.css', 'r') as f:
    css = f.read()

# Replace color variables (Cyberpunk / 16-bit arcade theme)
css = re.sub(r'--cream:\s*#[0-9a-fA-F]+;', '--cream: #090914;', css)
css = re.sub(r'--paper:\s*#[0-9a-fA-F]+;', '--paper: #121226;', css)
css = re.sub(r'--red:\s*#[0-9a-fA-F]+;', '--red: #ff007f;', css)
css = re.sub(r'--blue:\s*#[0-9a-fA-F]+;', '--blue: #00f3ff;', css)
css = re.sub(r'--green:\s*#[0-9a-fA-F]+;', '--green: #39ff14;', css)
css = re.sub(r'--orange:\s*#[0-9a-fA-F]+;', '--orange: #ffe600;', css)
css = re.sub(r'--pink:\s*#[0-9a-fA-F]+;', '--pink: #b026ff;', css)
css = re.sub(r'--ink:\s*#[0-9a-fA-F]+;', '--ink: #ffffff;', css)
css = re.sub(r'--yellow:\s*#[0-9a-fA-F]+;', '--yellow: #ff007f;', css)
css = re.sub(r'--border:\s*#[0-9a-fA-F]+;', '--border: #00f3ff;', css)

# Fix background SVGs (remove them)
css = re.sub(r'background-image:\s*url\([^)]+\);', 'background-image: none;', css)

# Replace fonts
css = re.sub(r'"Baloo 2",\s*system-ui,\s*sans-serif', '"VT323", monospace', css)
css = css.replace('"Baloo 2",sans-serif', '"VT323", monospace')
css = re.sub(r'"Shrikhand",\s*cursive', '"Press Start 2P", monospace', css)
css = re.sub(r'"Space Mono",\s*monospace', '"VT323", monospace', css)

# Remove border radius completely for blocky 16-bit feel
css = re.sub(r'border-radius:\s*[^;]+;', 'border-radius: 0;', css)

# Ensure text sizes are adjusted for the new fonts (Press Start 2P is very wide)
css = css.replace('font-size: clamp(3.75rem,7.5vw,7rem);', 'font-size: clamp(2.5rem,5vw,5rem);')
css = css.replace('font-size: clamp(2.7rem,5vw,4.8rem);', 'font-size: clamp(1.8rem,3vw,3.5rem);')
css = css.replace('font-size: clamp(2rem,3.4vw,3.1rem);', 'font-size: clamp(1.5rem,2.5vw,2.5rem);')
css = css.replace('font-size: clamp(3.45rem,16vw,5rem);', 'font-size: clamp(2rem,10vw,3rem);')
css = css.replace('font-size: clamp(2.7rem,5vw,4.7rem);', 'font-size: clamp(1.8rem,3vw,3.5rem);')

# Increase body font size slightly for VT323
css = css.replace('font-size: .73rem;', 'font-size: 1rem;')
css = css.replace('font-size: .72rem;', 'font-size: 1rem;')
css = css.replace('font-size: .71rem;', 'font-size: 1rem;')
css = css.replace('font-size: .76rem;', 'font-size: 1.1rem;')
css = css.replace('font-size: .75rem;', 'font-size: 1.1rem;')
css = css.replace('font-size: .77rem;', 'font-size: 1.1rem;')
css = css.replace('font-size: .84em;', 'font-size: 1.1em;')
css = css.replace('font-size: .9rem;', 'font-size: 1.2rem;')
css = css.replace('font-size: .94rem;', 'font-size: 1.2rem;')
css = css.replace('font-size: 1.1rem;', 'font-size: 1.4rem;')
css = css.replace('font-size: 1.16rem;', 'font-size: 1.5rem;')

# Add CRT text shadow to h1, h2
css = css.replace('.hero h1 {', '.hero h1 { text-shadow: 2px 2px 0 var(--pink), -2px -2px 0 var(--blue); ')
css = css.replace('.section-head h2 {', '.section-head h2 { text-shadow: 2px 2px 0 var(--pink); ')

# Fix specific text colors that clash with dark mode
css = css.replace('color: #ffeeb4;', 'color: var(--ink);')
css = css.replace('color: #e8e9d8;', 'color: var(--ink);')
css = css.replace('background: #fff8dc;', 'background: var(--paper);')
css = css.replace('background: #f8b1be;', 'background: var(--red);')
css = css.replace('background: #c7e9cc;', 'background: var(--green);')
css = css.replace('background: #ffc1ce;', 'background: var(--pink);')

# Fix code syntax highlighting colors for dark mode (already fine, but let's make them neon)
css = css.replace('color: #ff8fb7;', 'color: #ff007f;')
css = css.replace('color: #8de2b0;', 'color: #39ff14;')
css = css.replace('color: #90d9ff;', 'color: #00f3ff;')
css = css.replace('color: #ffe28b;', 'color: #ffe600;')

# Make the sun a retro grid or block
css = css.replace('.sun { position: absolute;', '.sun { position: absolute; border-radius: 0; ')

# Dark mode text needs to override some hardcoded colors
css = css.replace('color: var(--ink);', 'color: var(--ink);')

with open('site.css', 'w') as f:
    f.write(css)
