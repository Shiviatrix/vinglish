import turtle
import math

# ==========================================
# Englist Visual Identity
# A procedural, mathematical logo generator
# ==========================================

# --- Color Palette ---
BG_COLOR = "#F8F4ED"    # Ivory
PRIMARY = "#1B2A6B"     # Deep Indigo
SECONDARY = "#1C7A4F"   # Emerald
ACCENT = "#B46A2C"      # Copper

def setup_canvas():
    """Initialize the canvas with the specified background color."""
    screen = turtle.Screen()
    screen.setup(width=1000, height=1000)
    screen.bgcolor(BG_COLOR)
    screen.title("Englist Language Visual Identity")
    screen.tracer(0, 0)  # Disable auto-update for instant rendering
    
    t = turtle.Turtle()
    t.hideturtle()
    t.speed(0)
    return screen, t

# ==========================================
# Geometric Primitives
# ==========================================

def draw_circle(t, x, y, radius, color, thickness=1, fill=False):
    """Draw a perfect circle centered at (x, y)."""
    t.penup()
    t.goto(x, y - radius)
    t.setheading(0)
    t.pendown()
    t.color(color)
    t.pensize(thickness)
    if fill:
        t.fillcolor(color)
        t.begin_fill()
    t.circle(radius)
    if fill:
        t.end_fill()

def draw_line(t, x1, y1, x2, y2, color, thickness=1):
    """Draw a straight line between two points."""
    t.penup()
    t.goto(x1, y1)
    t.pendown()
    t.color(color)
    t.pensize(thickness)
    t.goto(x2, y2)

def draw_polygon(t, points, color, thickness=1, fill=False):
    """Draw a polygon from a list of (x, y) tuples."""
    if not points: return
    t.penup()
    t.goto(points[0])
    t.pendown()
    t.color(color)
    t.pensize(thickness)
    if fill:
        t.fillcolor(color)
        t.begin_fill()
    for p in points[1:]:
        t.goto(p)
    t.goto(points[0])
    if fill:
        t.end_fill()

def draw_arc(t, x, y, radius, start_angle, extent, color, thickness=1):
    """Draw an arc centered at (x, y)."""
    t.penup()
    # Move to the start of the arc
    start_rad = math.radians(start_angle)
    start_x = x + radius * math.cos(start_rad)
    start_y = y + radius * math.sin(start_rad)
    t.goto(start_x, start_y)
    t.setheading(start_angle + 90)
    t.pendown()
    t.color(color)
    t.pensize(thickness)
    t.circle(radius, extent)

# ==========================================
# Mathematical Helpers
# ==========================================

def get_point_on_circle(x, y, radius, angle_deg):
    """Calculate the (x, y) coordinate on a circle at a given angle."""
    rad = math.radians(angle_deg)
    return (x + radius * math.cos(rad), y + radius * math.sin(rad))

def create_star_polygon(center_x, center_y, outer_radius, inner_radius, points):
    """Generate vertices for a star polygon."""
    vertices = []
    angle_step = 360 / (points * 2)
    for i in range(points * 2):
        r = outer_radius if i % 2 == 0 else inner_radius
        vertices.append(get_point_on_circle(center_x, center_y, r, i * angle_step))
    return vertices

def create_regular_polygon(center_x, center_y, radius, sides, rotation=0):
    """Generate vertices for a regular polygon."""
    vertices = []
    angle_step = 360 / sides
    for i in range(sides):
        vertices.append(get_point_on_circle(center_x, center_y, radius, rotation + i * angle_step))
    return vertices

# ==========================================
# Structural Layers
# ==========================================

def draw_outer_ring(t):
    """
    Draws the outermost boundary representing the global scope and 
    the mathematically precise foundation of the language.
    """
    center = (0, 0)
    outer_r = 400
    
    # Base bounding circles
    draw_circle(t, center[0], center[1], outer_r, PRIMARY, thickness=3)
    draw_circle(t, center[0], center[1], outer_r - 12, PRIMARY, thickness=1)
    draw_circle(t, center[0], center[1], outer_r - 20, PRIMARY, thickness=1)
    
    # Precision ticks (like an astrolabe or compiler clock)
    num_ticks = 120
    for i in range(num_ticks):
        angle = i * (360 / num_ticks)
        length = 8 if i % 5 == 0 else 4
        thickness = 2 if i % 5 == 0 else 1
        color = ACCENT if i % 15 == 0 else PRIMARY
        
        p1 = get_point_on_circle(0, 0, outer_r - 12, angle)
        p2 = get_point_on_circle(0, 0, outer_r - 12 + length, angle)
        draw_line(t, p1[0], p1[1], p2[0], p2[1], color, thickness)
        
    # Subtle inner dashed ring
    dash_count = 72
    for i in range(dash_count):
        start_angle = i * (360 / dash_count)
        draw_arc(t, 0, 0, outer_r - 35, start_angle, 2.5, SECONDARY, thickness=1)

def draw_compiler_layers(t):
    """
    Represents the compilation pipeline: 
    Source -> AST -> HIR -> MIR -> Machine Code.
    Drawn as nesting octagons (8 representing bytes/bits).
    """
    radii = [340, 310, 280, 250, 220]
    
    for i, r in enumerate(radii):
        # Rotate alternating layers for geometric weaving
        rot = 0 if i % 2 == 0 else 22.5
        poly = create_regular_polygon(0, 0, r, 8, rotation=rot)
        
        # Thicker line for the machine code layer (innermost)
        thick = 2 if i == len(radii)-1 else 1
        
        # Accentuate the HIR/MIR boundary
        color = SECONDARY if i == 2 else PRIMARY
        draw_polygon(t, poly, color, thickness=thick)
        
        # Draw connecting lines between layers to show pipeline flow
        if i < len(radii) - 1:
            next_rot = 22.5 if i % 2 == 0 else 0
            next_poly = create_regular_polygon(0, 0, radii[i+1], 8, rotation=next_rot)
            for j in range(8):
                # Connect midpoints of current to vertices of next
                p1 = poly[j]
                p2 = next_poly[j]
                draw_line(t, p1[0], p1[1], p2[0], p2[1], PRIMARY, thickness=1)

def draw_kolam_weave(t):
    """
    A continuous looping path reflecting South Indian Kolam art.
    Represents the syntax, human intent, and grammatical elegance.
    """
    # Kolam base nodes (dots)
    nodes = []
    r_nodes = 160
    for i in range(12):
        nodes.append(get_point_on_circle(0, 0, r_nodes, i * 30))
        
    for p in nodes:
        draw_circle(t, p[0], p[1], 4, ACCENT, fill=True)
        
    # Looping curves around the nodes
    t.color(PRIMARY)
    t.pensize(2)
    
    steps = 360
    t.penup()
    for i in range(steps + 1):
        # Parametric equation for a continuous overlapping floral loop (rose curve variation)
        theta = math.radians(i)
        
        # 12-petaled loop that weaves around the nodes
        r = 160 + 45 * math.sin(6 * theta)
        x = r * math.cos(theta)
        y = r * math.sin(theta)
        
        if i == 0:
            t.goto(x, y)
            t.pendown()
        else:
            t.goto(x, y)
            
    # Secondary inner weave
    t.penup()
    t.color(SECONDARY)
    t.pensize(1)
    for i in range(steps + 1):
        theta = math.radians(i)
        r = 130 + 20 * math.cos(12 * theta)
        x = r * math.cos(theta)
        y = r * math.sin(theta)
        
        if i == 0:
            t.goto(x, y)
            t.pendown()
        else:
            t.goto(x, y)

def draw_mandala_symmetry(t):
    """
    Interlocking seed-of-life / mandala geometry representing 
    the modularity and harmony of the standard library.
    """
    r_center = 0
    r_circles = 60
    distance = 60
    
    # 6 interlocking circles
    for i in range(6):
        angle = i * 60
        center = get_point_on_circle(0, 0, distance, angle)
        draw_circle(t, center[0], center[1], r_circles, PRIMARY, thickness=1)
        
    # Outer bounding ring for the mandala
    draw_circle(t, 0, 0, distance + r_circles, PRIMARY, thickness=2)
    draw_circle(t, 0, 0, distance + r_circles + 5, SECONDARY, thickness=1)

def draw_central_E_abstraction(t):
    """
    The central abstract "E" representing Englist.
    Constructed using pure geometry (horizontal bars aligned to a vertical spine).
    Designed to look like a temple pillar or abstract formal grammar symbol.
    """
    spine_x = -20
    
    # Vertical Spine
    draw_line(t, spine_x, 40, spine_x, -40, PRIMARY, thickness=5)
    
    # Top bar
    draw_line(t, spine_x, 40, spine_x + 40, 40, PRIMARY, thickness=5)
    # Middle bar (slightly shorter)
    draw_line(t, spine_x, 0, spine_x + 30, 0, PRIMARY, thickness=5)
    # Bottom bar
    draw_line(t, spine_x, -40, spine_x + 40, -40, PRIMARY, thickness=5)
    
    # Geometric endpoints (intent nodes)
    draw_circle(t, spine_x + 40, 40, 4, ACCENT, fill=True)
    draw_circle(t, spine_x + 30, 0, 4, ACCENT, fill=True)
    draw_circle(t, spine_x + 40, -40, 4, ACCENT, fill=True)
    draw_circle(t, spine_x, 40, 4, ACCENT, fill=True)
    draw_circle(t, spine_x, -40, 4, ACCENT, fill=True)

def draw_accents(t):
    """
    Adds small geometric nodes and subtle accents to elevate the design.
    """
    # 4 Cardinal direction diamonds
    for i in range(4):
        angle = i * 90
        p = get_point_on_circle(0, 0, 375, angle)
        
        # Draw a small diamond
        diamond = [
            (p[0], p[1] + 8),
            (p[0] + 8, p[1]),
            (p[0], p[1] - 8),
            (p[0] - 8, p[1])
        ]
        draw_polygon(t, diamond, ACCENT, thickness=1, fill=True)

def main():
    screen, t = setup_canvas()
    
    # 1. Outer foundation (The Environment / Global Scope)
    draw_outer_ring(t)
    
    # 2. Compilation Stages (Octagonal Layers)
    draw_compiler_layers(t)
    
    # 3. Syntax and Intent (Kolam continuous loop)
    draw_kolam_weave(t)
    
    # 4. Standard Library Modularity (Mandala / Seed of life)
    draw_mandala_symmetry(t)
    
    # 5. Core Identifier (Abstract "E")
    draw_central_E_abstraction(t)
    
    # 6. Final Polish (Nodes and Accents)
    draw_accents(t)
    
    # Render to screen
    screen.update()
    
    # Save as EPS file
    screen.getcanvas().postscript(file="englist_logo.eps")
    print("Logo saved to englist_logo.eps")
    
    # Keep window open
    turtle.done()

if __name__ == "__main__":
    main()
