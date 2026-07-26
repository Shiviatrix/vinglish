"""Generate the type system diagram as SVG using graphviz."""

import graphviz

g = graphviz.Digraph(
    "type_system",
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
        "shape": "record",
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
        "style": "solid",
    },
)

g.node(
    "type",
    "Type|"
    "Int (number)\\l"
    "Float (decimal)\\l"
    "Bool (boolean)\\l"
    "Text (text)\\l"
    "Unit\\l"
    "Reference(Box\\<Type\\>, bool)\\l"
    "Pointer(Box\\<Type\\>)\\l"
    "List(Box\\<Type\\>)\\l"
    "Dict(Box\\<Type\\>, Box\\<Type\\>)\\l"
    "Optional(Box\\<Type\\>)\\l"
    "Result(Box\\<Type\\>, Box\\<Type\\>)\\l"
    "Function(Vec\\<Type\\>, Box\\<Type\\>)\\l"
    "Named(String, Vec\\<Type\\>)\\l"
    "Var(TypeVar)\\l",
)

g.node("typevar", "TypeVar|id: u32\\l(AtomicU32 counter)\\l")

g.node(
    "copy",
    "Copy Semantics|"
    "Int\\l"
    "Float\\l"
    "Bool\\l"
    "Text\\l"
    "Unit\\l"
    "Pointer\\l",
)

g.node(
    "move",
    "Move Semantics|"
    "List\\l"
    "Dict\\l"
    "Optional\\l"
    "Result\\l"
    "Named (structs)\\l"
    "Reference\\l"
    "Function\\l",
)

g.edge("type", "typevar", label="Var")
g.edge("type", "copy", label="is_copy() = true", style="dashed")
g.edge("type", "move", label="is_copy() = false", style="dashed")

g.render("type_system", directory=".", cleanup=True)
print("Generated type_system.svg")
