"""Generate the MIR data flow diagram as SVG using graphviz."""

import graphviz

g = graphviz.Digraph(
    "mir_data_flow",
    format="svg",
    graph_attr={
        "bgcolor": "white",
        "rankdir": "TB",
        "fontname": "Helvetica",
        "fontsize": "11",
        "nodesep": "0.4",
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
    },
)

g.node("module", "MirModule\\<V\\>|functions: Vec\\<MirFunction\\<V\\>\\>")
g.node("func", "MirFunction\\<V\\>|id: FunctionId\\lname: String\\lparams: Vec\\<V\\>\\llocals: Vec\\<V\\>\\lblocks: Vec\\<BasicBlock\\<V\\>\\>\\l")
g.node("block", "BasicBlock\\<V\\>|id: BlockId\\linstrs: Vec\\<Instruction\\<V\\>\\>\\lterminator: Terminator\\<V\\>\\l")

g.node(
    "instr",
    "Instruction\\<V\\>|"
    "Assign\\l"
    "LoadField\\l"
    "StoreField\\l"
    "Call\\l"
    "CallIntrinsic\\l"
    "HeapAllocate\\l"
    "StackAllocate\\l"
    "BinaryOp\\l"
    "UnaryOp\\l"
    "Borrow / BorrowMut\\l"
    "Deref\\l"
    "Drop\\l"
    "Phi\\l",
)

g.node(
    "term",
    "Terminator\\<V\\>|"
    "Return(Option\\<Operand\\>)\\l"
    "Jump(BlockId)\\l"
    "Branch(Operand, BlockId, BlockId)\\l",
)

g.node("operand", "Operand\\<V\\>|Constant(Literal)\\lVar(V)\\l")

g.edge("module", "func", label="1..*")
g.edge("func", "block", label="1..*")
g.edge("block", "instr", label="0..*")
g.edge("block", "term", label="1")
g.edge("instr", "operand", label="uses")
g.edge("term", "operand", label="uses")

g.render("mir_data_flow", directory=".", cleanup=True)
print("Generated mir_data_flow.svg")
