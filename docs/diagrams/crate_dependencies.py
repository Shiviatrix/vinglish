"""Generate the crate dependency diagram as SVG using graphviz."""

import graphviz

g = graphviz.Digraph(
    "crate_dependencies",
    format="svg",
    graph_attr={
        "bgcolor": "white",
        "rankdir": "TB",
        "fontname": "Helvetica",
        "fontsize": "11",
        "nodesep": "0.5",
        "ranksep": "0.4",
        "margin": "0.2",
    },
    node_attr={
        "shape": "box",
        "style": "solid",
        "fontname": "Helvetica",
        "fontsize": "9",
        "color": "black",
        "fillcolor": "white",
        "penwidth": "1.0",
    },
    edge_attr={
        "color": "black",
        "arrowsize": "0.6",
        "penwidth": "0.7",
    },
)

# Crate nodes
crates = [
    "vinglish-cli",
    "vinglish-lexer",
    "vinglish-parser",
    "vinglish-hir",
    "vinglish-types",
    "vinglish-mir",
    "vinglish-ssa",
    "vinglish-opt",
    "vinglish-codegen",
    "vinglish-own",
    "vinglish-ownership",
    "vinglish-diagnostics",
    "vinglish-decompile",
    "vinglish-ir-export",
    "vinglish-fmt",
    "vinglish-lsp",
    "vinglish-llvm",
    "vinglish-macro",
    "vinglish-analysis",
]

for c in crates:
    g.node(c, c)

# Dependencies (derived from imports in source files and Cargo.toml structure)
deps = [
    ("vinglish-cli", "vinglish-lexer"),
    ("vinglish-cli", "vinglish-parser"),
    ("vinglish-cli", "vinglish-codegen"),
    ("vinglish-cli", "vinglish-diagnostics"),
    ("vinglish-cli", "vinglish-fmt"),
    ("vinglish-cli", "vinglish-hir"),
    ("vinglish-cli", "vinglish-ir-export"),
    ("vinglish-cli", "vinglish-lsp"),
    ("vinglish-cli", "vinglish-mir"),
    ("vinglish-cli", "vinglish-opt"),
    ("vinglish-cli", "vinglish-own"),
    ("vinglish-cli", "vinglish-ownership"),
    ("vinglish-cli", "vinglish-ssa"),
    ("vinglish-cli", "vinglish-types"),
    ("vinglish-cli", "vinglish-llvm"),
    ("vinglish-parser", "vinglish-lexer"),
    ("vinglish-hir", "vinglish-lexer"),
    ("vinglish-hir", "vinglish-parser"),
    ("vinglish-types", "vinglish-hir"),
    ("vinglish-types", "vinglish-lexer"),
    ("vinglish-types", "vinglish-parser"),
    ("vinglish-types", "vinglish-mir"),
    ("vinglish-mir", "vinglish-hir"),
    ("vinglish-mir", "vinglish-parser"),
    ("vinglish-ssa", "vinglish-hir"),
    ("vinglish-ssa", "vinglish-mir"),
    ("vinglish-opt", "vinglish-hir"),
    ("vinglish-opt", "vinglish-mir"),
    ("vinglish-codegen", "vinglish-hir"),
    ("vinglish-codegen", "vinglish-mir"),
    ("vinglish-codegen", "vinglish-parser"),
    ("vinglish-codegen", "vinglish-decompile"),
    ("vinglish-own", "vinglish-hir"),
    ("vinglish-own", "vinglish-mir"),
    ("vinglish-own", "vinglish-diagnostics"),
    ("vinglish-ownership", "vinglish-parser"),
    ("vinglish-diagnostics", "vinglish-lexer"),
    ("vinglish-llvm", "vinglish-hir"),
    ("vinglish-llvm", "vinglish-mir"),
    ("vinglish-ir-export", "vinglish-hir"),
]

for src, dst in deps:
    g.edge(src, dst)

g.render("crate_dependencies", directory=".", cleanup=True)
print("Generated crate_dependencies.svg")
