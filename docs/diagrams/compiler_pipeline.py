"""Generate the compiler pipeline diagram as SVG using graphviz."""

import graphviz

g = graphviz.Digraph(
    "compiler_pipeline",
    format="svg",
    graph_attr={
        "bgcolor": "white",
        "rankdir": "TB",
        "fontname": "Helvetica",
        "fontsize": "11",
        "nodesep": "0.4",
        "ranksep": "0.35",
        "margin": "0.2",
    },
    node_attr={
        "shape": "box",
        "style": "solid",
        "fontname": "Helvetica",
        "fontsize": "10",
        "color": "black",
        "fillcolor": "white",
        "penwidth": "1.0",
    },
    edge_attr={
        "color": "black",
        "arrowsize": "0.7",
        "penwidth": "0.8",
    },
)

stages = [
    ("src", "Source (.ving)"),
    ("lex", "tokenize()\n[vinglish-lexer]"),
    ("parse", "parse()\n[vinglish-parser]"),
    ("nameres", "NameResolutionPass\n[vinglish-types]"),
    ("typeinf", "TypeInferencePass\n[vinglish-types]"),
    ("hirval", "HirValidatorPass\n[vinglish-types]"),
    ("ownast", "check_module()\n[vinglish-ownership]"),
    ("mirlower", "MirLowerer\n[vinglish-types]"),
    ("mirval", "MirValidatorPass\n[vinglish-mir]"),
    ("pressa", "Pre-SSA Pipeline\n[vinglish-opt]"),
    ("ssa", "SSAConversionPass\n[vinglish-ssa]"),
    ("ssaval", "SSAValidator\n[vinglish-ssa]"),
    ("postssa", "Post-SSA Pipeline\n[vinglish-opt]"),
    ("ownmir", "OwnershipAnalysis\n[vinglish-own]"),
    ("ownval", "OwnershipValidator\n[vinglish-own]"),
    ("codegen", "emit_mir_c()\n[vinglish-codegen]"),
    ("cc", "System C Compiler"),
    ("bin", "Native Binary"),
]

for node_id, label in stages:
    g.node(node_id, label)

for i in range(len(stages) - 1):
    g.edge(stages[i][0], stages[i + 1][0])

# Data type annotations on edges
g.edge("lex", "parse", label="Vec<Spanned<Token>>", fontsize="8", fontname="Helvetica")
g.edge("parse", "nameres", label="ast::Module", fontsize="8", fontname="Helvetica")
g.edge("typeinf", "hirval", label="hir::Module", fontsize="8", fontname="Helvetica")
g.edge("mirlower", "mirval", label="MirModule<VariableId>", fontsize="8", fontname="Helvetica")
g.edge("ssa", "ssaval", label="MirModule<SsaValueId>", fontsize="8", fontname="Helvetica")
g.edge("codegen", "cc", label="C source", fontsize="8", fontname="Helvetica")

g.render("compiler_pipeline", directory=".", cleanup=True)
print("Generated compiler_pipeline.svg")
