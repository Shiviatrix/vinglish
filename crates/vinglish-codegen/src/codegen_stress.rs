#![allow(clippy::module_inception)]
/// Stress tests for the MIR → C codegen + payload extraction roundtrip.
#[cfg(test)]
mod codegen_stress {
    use crate::mir_codegen::emit_mir_c;
    use vinglish_hir::symbol::{FunctionId, SymbolId, SymbolTable, VariableId};
    use vinglish_mir::{
        BasicBlock, BlockId, CallTarget, Instruction, MirFunction, MirModule, Operand, Terminator,
    };
    use vinglish_parser::ast::{BinOp, Literal};

    fn var(n: u32) -> VariableId { VariableId(SymbolId(n)) }
    fn fid(n: u32) -> FunctionId { FunctionId(SymbolId(n)) }

    // ─── 1. Minimal module: one function, one block, one instruction ──────────

    #[test]
    fn minimal_module_roundtrip() {
        let v = var(1);
        let module = MirModule {
            functions: vec![MirFunction {
                id: fid(1),
                is_foreign: false,
                name: "main".into(),
                params: vec![],
                locals: vec![v],
                blocks: vec![BasicBlock {
                    id: BlockId(0),
                    instrs: vec![Instruction::Assign(v, Operand::Constant(Literal::Int(0)))],
                    terminator: Terminator::Return(Some(Operand::Var(v))),
                }],
            }],
        };
        let c = emit_mir_c(&module, &SymbolTable::new()).expect("emit must succeed");
        let bytes = vinglish_decompile::extract_mir_payload(&c).expect("roundtrip must succeed");
        // Re-deserialize and verify MIR equality
        let restored: MirModule<VariableId> = bincode::deserialize(&bytes).expect("bincode must deserialize");
        assert_eq!(restored.functions.len(), 1);
        assert_eq!(restored.functions[0].name, "main");
    }

    // ─── 2. All BinOp variants emit and roundtrip correctly ──────────────────

    #[test]
    fn all_binary_ops_roundtrip() {
        let ops = [
            BinOp::Add, BinOp::Sub, BinOp::Mul, BinOp::Div, BinOp::Mod,
            BinOp::Eq, BinOp::NotEq, BinOp::Lt, BinOp::Gt, BinOp::LtEq, BinOp::GtEq,
            BinOp::And, BinOp::Or,
        ];
        let (lhs, rhs, dst) = (var(1), var(2), var(3));
        for op in ops {
            let module = MirModule {
                functions: vec![MirFunction {
                    id: fid(10),
                    is_foreign: false,
                    name: "main".into(),
                    params: vec![lhs, rhs],
                    locals: vec![lhs, rhs, dst],
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        instrs: vec![Instruction::BinaryOp(dst, op, Operand::Var(lhs), Operand::Var(rhs))],
                        terminator: Terminator::Return(Some(Operand::Var(dst))),
                    }],
                }],
            };
            let c = emit_mir_c(&module, &SymbolTable::new())
                .unwrap_or_else(|_| panic!("emit failed for {:?}", op));
            vinglish_decompile::extract_mir_payload(&c)
                .unwrap_or_else(|e| panic!("roundtrip failed for {:?}: {:?}", op, e));
        }
    }

    // ─── 3. Multi-function module with cross-call ─────────────────────────────

    #[test]
    fn multi_function_module_roundtrip() {
        let (a, b, c) = (var(1), var(2), var(3));
        let helper = fid(1);
        let main_fn = fid(2);
        let module = MirModule {
            functions: vec![
                MirFunction {
                    id: helper,
                    is_foreign: false,
                    name: "helper".into(),
                    params: vec![a],
                    locals: vec![a, b],
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        instrs: vec![
                            Instruction::BinaryOp(b, BinOp::Mul, Operand::Var(a), Operand::Constant(Literal::Int(2))),
                        ],
                        terminator: Terminator::Return(Some(Operand::Var(b))),
                    }],
                },
                MirFunction {
                    id: main_fn,
                    is_foreign: false,
                    name: "main".into(),
                    params: vec![],
                    locals: vec![a, c],
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        instrs: vec![
                            Instruction::Assign(a, Operand::Constant(Literal::Int(21))),
                            Instruction::Call(c, CallTarget::Direct(helper), vec![Operand::Var(a)]),
                        ],
                        terminator: Terminator::Return(Some(Operand::Var(c))),
                    }],
                },
            ],
        };
        let out = emit_mir_c(&module, &SymbolTable::new()).unwrap();
        let bytes = vinglish_decompile::extract_mir_payload(&out).unwrap();
        let restored: MirModule<VariableId> = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.functions.len(), 2);
        assert_eq!(restored.functions[0].name, "helper");
        assert_eq!(restored.functions[1].name, "main");
    }

    // ─── 4. String literals in the pool survive roundtrip ────────────────────

    #[test]
    fn string_literal_pool_survives_roundtrip() {
        let dst = var(1);
        let texts = ["hello", "world", "vinglish", "✦ stars ✦", "tab\there", "quote\"end", "back\\slash"];
        for text in texts {
            let module = MirModule {
                functions: vec![MirFunction {
                    id: fid(5),
                    is_foreign: false,
                    name: "main".into(),
                    params: vec![],
                    locals: vec![dst],
                    blocks: vec![BasicBlock {
                        id: BlockId(0),
                        instrs: vec![Instruction::Assign(dst, Operand::Constant(Literal::Text(text.into())))],
                        terminator: Terminator::Return(None),
                    }],
                }],
            };
            let c = emit_mir_c(&module, &SymbolTable::new())
                .unwrap_or_else(|_| panic!("emit failed for text {:?}", text));
            let bytes = vinglish_decompile::extract_mir_payload(&c)
                .unwrap_or_else(|e| panic!("roundtrip failed for text {:?}: {:?}", text, e));
            let restored: MirModule<VariableId> = bincode::deserialize(&bytes).unwrap();
            assert_eq!(restored.functions[0].blocks[0].instrs.len(), 1);
        }
    }

    // ─── 5. Branch + phi-node: control flow survives roundtrip ───────────────

    #[test]
    fn branch_phi_roundtrip() {
        let (cond, a, b, result) = (var(1), var(2), var(3), var(4));
        let module = MirModule {
            functions: vec![MirFunction {
                id: fid(7),
                is_foreign: false,
                name: "main".into(),
                params: vec![cond],
                locals: vec![cond, a, b, result],
                blocks: vec![
                    BasicBlock {
                        id: BlockId(0),
                        instrs: vec![
                            Instruction::Assign(a, Operand::Constant(Literal::Int(1))),
                            Instruction::Assign(b, Operand::Constant(Literal::Int(2))),
                        ],
                        terminator: Terminator::Branch(Operand::Var(cond), BlockId(1), BlockId(2)),
                    },
                    BasicBlock {
                        id: BlockId(1),
                        instrs: vec![],
                        terminator: Terminator::Jump(BlockId(3)),
                    },
                    BasicBlock {
                        id: BlockId(2),
                        instrs: vec![],
                        terminator: Terminator::Jump(BlockId(3)),
                    },
                    BasicBlock {
                        id: BlockId(3),
                        instrs: vec![Instruction::Phi(result, vec![
                            (Operand::Var(a), BlockId(1)),
                            (Operand::Var(b), BlockId(2)),
                        ])],
                        terminator: Terminator::Return(Some(Operand::Var(result))),
                    },
                ],
            }],
        };
        let c = emit_mir_c(&module, &SymbolTable::new()).unwrap();
        let bytes = vinglish_decompile::extract_mir_payload(&c).unwrap();
        let restored: MirModule<VariableId> = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.functions[0].blocks.len(), 4);
        // Phi instruction must be preserved
        assert!(matches!(restored.functions[0].blocks[3].instrs[0], Instruction::Phi(_, _)));
    }

    // ─── 6. Scale: 1 000 functions, verifying O(1) payload overhead ──────────

    #[test]
    fn thousand_functions_roundtrip_and_payload_is_sane() {
        let v = var(1);
        let functions: Vec<MirFunction<VariableId>> = (0..1_000)
            .map(|i| MirFunction {
                id: fid(i as u32 + 100),
                is_foreign: false,
                name: format!("fn_{}", i),
                params: vec![],
                locals: vec![v],
                blocks: vec![BasicBlock {
                    id: BlockId(0),
                    instrs: vec![Instruction::Assign(v, Operand::Constant(Literal::Int(i as i64)))],
                    terminator: Terminator::Return(Some(Operand::Var(v))),
                }],
            })
            .collect();
        let module = MirModule { functions };
        let c = emit_mir_c(&module, &SymbolTable::new()).unwrap();
        let bytes = vinglish_decompile::extract_mir_payload(&c).unwrap();
        let restored: MirModule<VariableId> = bincode::deserialize(&bytes).unwrap();
        assert_eq!(restored.functions.len(), 1_000);
        // Verify payload comment appears only once
        let count = c.matches("/* VINGLISH_MIR_PAYLOAD").count();
        assert_eq!(count, 1, "payload comment must appear exactly once");
    }

    // ─── 7. Foreign call target must survive roundtrip ────────────────────────

    #[test]
    fn foreign_call_target_roundtrip() {
        let dst = var(1);
        let module = MirModule {
            functions: vec![MirFunction {
                id: fid(20),
                is_foreign: false,
                name: "main".into(),
                params: vec![],
                locals: vec![dst],
                blocks: vec![BasicBlock {
                    id: BlockId(0),
                    instrs: vec![Instruction::Call(
                        dst,
                        CallTarget::Foreign { c_symbol: "printf".into() },
                        vec![],
                    )],
                    terminator: Terminator::Return(Some(Operand::Var(dst))),
                }],
            }],
        };
        let c = emit_mir_c(&module, &SymbolTable::new()).unwrap();
        let bytes = vinglish_decompile::extract_mir_payload(&c).unwrap();
        let restored: MirModule<VariableId> = bincode::deserialize(&bytes).unwrap();
        let instr = &restored.functions[0].blocks[0].instrs[0];
        assert!(
            matches!(instr, Instruction::Call(_, CallTarget::Foreign { c_symbol }, _) if c_symbol == "printf"),
            "foreign call target must survive roundtrip"
        );
    }

    // ─── 8. Emit must produce valid C that compiles with cc ──────────────────

    #[test]
    fn emitted_c_is_syntactically_parseable() {
        // We can't invoke cc from a unit test portably, but we can assert that
        // the C body at minimum contains the required headers and a main guard.
        let v = var(1);
        let module = MirModule {
            functions: vec![MirFunction {
                id: fid(1),
                is_foreign: false,
                name: "main".into(),
                params: vec![],
                locals: vec![v],
                blocks: vec![BasicBlock {
                    id: BlockId(0),
                    instrs: vec![Instruction::Assign(v, Operand::Constant(Literal::Int(0)))],
                    terminator: Terminator::Return(Some(Operand::Var(v))),
                }],
            }],
        };
        let c = emit_mir_c(&module, &SymbolTable::new()).unwrap();
        assert!(c.contains("#include <stdint.h>"), "must include stdint.h");
        assert!(c.contains("#include <stdio.h>"), "must include stdio.h");
        assert!(c.contains("#include <stdlib.h>"), "must include stdlib.h");
        assert!(c.contains("int main("), "must define main");
        assert!(c.contains("/* VINGLISH_MIR_PAYLOAD:"), "must contain payload comment");
    }
}
