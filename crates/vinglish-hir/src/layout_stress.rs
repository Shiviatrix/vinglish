#![allow(clippy::module_inception)]
/// Stress tests for the C ABI layout resolver.
///
/// Exercises:
/// - Alignment arithmetic (`align_up`) for all power-of-two alignments
/// - LP64 scalar sizes match the SysV/macOS ABI contract
/// - Single-field structs: size == field size, offset == 0
/// - Multi-field structs: correct offsets between fields
/// - Field offsets are strictly monotonically increasing
/// - Recursive-by-value detection (must error, never stack overflow)
/// - Unknown type detection
/// - Record with 100 fields: offset progression and size are correct
#[cfg(test)]
mod layout_stress {
    use crate::layout::{CAbi, LayoutError, LayoutResolver, align_up};
    use crate::symbol::{SymbolId, SymbolTable, TypeId, TypeSymbol};
    use crate::types::Type;
    use vinglish_parser::ast::Visibility;

    fn resolver<'a>(symbols: &'a SymbolTable) -> LayoutResolver<'a> {
        LayoutResolver::new(symbols, CAbi::LP64)
    }

    fn pub_vis() -> Visibility { Visibility::Public }

    /// Register a named type with `n` consecutive integer fields and return its TypeId.
    fn register_n_int_fields(symbols: &mut SymbolTable, name: &str, n: usize) -> TypeId {
        // We need the TypeId before constructing the TypeSymbol.
        // Use a placeholder id then define.
        let placeholder_id = TypeId(SymbolId(symbols.num_symbols() as u32));
        let mut sym = TypeSymbol::new(placeholder_id, name.to_string(), pub_vis());
        for i in 0..n {
            sym.add_field(format!("f{}", i), Type::Int, pub_vis());
        }
        symbols.define_type(name.to_string(), sym)
    }

    // ─── 1. align_up exhaustive ───────────────────────────────────────────────

    #[test]
    fn align_up_returns_zero_for_zero_value() {
        for align in [1u32, 2, 4, 8, 16, 64] {
            assert_eq!(align_up(0, align), 0, "align_up(0, {}) must be 0", align);
        }
    }

    #[test]
    fn align_up_already_aligned_unchanged() {
        assert_eq!(align_up(8, 8), 8);
        assert_eq!(align_up(16, 8), 16);
        assert_eq!(align_up(64, 8), 64);
        assert_eq!(align_up(1, 1), 1);
    }

    #[test]
    fn align_up_rounds_to_next_boundary() {
        assert_eq!(align_up(9, 8), 16);
        assert_eq!(align_up(1, 8), 8);
        assert_eq!(align_up(7, 8), 8);
        assert_eq!(align_up(15, 8), 16);
        assert_eq!(align_up(17, 16), 32);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(3, 4), 4);
        assert_eq!(align_up(5, 4), 8);
    }

    #[test]
    fn align_up_all_byte_values_1_to_255_align_8() {
        for val in 1u32..=255 {
            let result = align_up(val, 8);
            assert!(result >= val, "align_up must not shrink");
            assert_eq!(result % 8, 0, "result must be 8-aligned");
            assert!(result <= val + 7, "must not overshoot by more than align-1");
        }
    }

    // ─── 2. LP64 ABI constants ────────────────────────────────────────────────

    #[test]
    fn lp64_abi_sizes_match_sysv() {
        let abi = CAbi::LP64;
        assert_eq!(abi.long_size, 8);
        assert_eq!(abi.long_align, 8);
        assert_eq!(abi.pointer_size, 8);
        assert_eq!(abi.pointer_align, 8);
        assert_eq!(abi.double_size, 8);
        assert_eq!(abi.double_align, 8);
    }

    // ─── 3. Unknown type returns LayoutError::UnknownType ────────────────────

    #[test]
    fn unknown_type_id_returns_error() {
        let symbols = SymbolTable::new();
        let r = resolver(&symbols);
        let bogus = TypeId(SymbolId(999));
        assert_eq!(r.layout_type(bogus), Err(LayoutError::UnknownType(bogus)));
    }

    // ─── 4. Single-field struct: offset=0, size=8 ────────────────────────────

    #[test]
    fn single_int_field_offset_0_size_8() {
        let mut symbols = SymbolTable::new();
        let tid = register_n_int_fields(&mut symbols, "Point", 1);
        let r = resolver(&symbols);
        let layout = r.layout_type(tid).unwrap();
        assert_eq!(layout.fields.len(), 1);
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[0].size, 8);
        assert_eq!(layout.size, 8);
        assert_eq!(layout.align, 8);
    }

    // ─── 5. Two-field struct: no padding needed ───────────────────────────────

    #[test]
    fn two_long_fields_offsets_0_and_8() {
        let mut symbols = SymbolTable::new();
        let tid = register_n_int_fields(&mut symbols, "P2D", 2);
        let r = resolver(&symbols);
        let layout = r.layout_type(tid).unwrap();
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[1].offset, 8);
        assert_eq!(layout.size, 16);
    }

    // ─── 6. Field offsets are strictly monotonically increasing ───────────────

    #[test]
    fn field_offsets_strictly_monotonic_10_fields() {
        let mut symbols = SymbolTable::new();
        let tid = register_n_int_fields(&mut symbols, "Big10", 10);
        let r = resolver(&symbols);
        let layout = r.layout_type(tid).unwrap();
        let offsets: Vec<u32> = layout.fields.iter().map(|f| f.offset).collect();
        for w in offsets.windows(2) {
            assert!(w[1] > w[0], "offsets not strictly monotonic: {:?}", offsets);
        }
    }

    // ─── 7. Size equals N * 8 for N homogeneous long fields ───────────────────

    #[test]
    fn size_equals_n_times_8_for_n_int_fields() {
        let mut symbols = SymbolTable::new();
        for n in [1usize, 3, 5, 7, 10] {
            let name = format!("T{}", n);
            let tid = register_n_int_fields(&mut symbols, &name, n);
            let r = resolver(&symbols);
            let layout = r.layout_type(tid).unwrap();
            assert_eq!(layout.size, (n as u32) * 8, "n={}: expected size {}", n, n * 8);
        }
    }

    // ─── 8. field_offset API agrees with layout fields ────────────────────────

    #[test]
    fn field_offset_api_matches_layout() {
        let mut symbols = SymbolTable::new();
        let tid = register_n_int_fields(&mut symbols, "Vec2", 2);
        let r = resolver(&symbols);
        let layout = r.layout_type(tid).unwrap();
        for fl in &layout.fields {
            assert_eq!(
                r.field_offset(tid, fl.id).unwrap(),
                fl.offset,
                "field_offset API must agree with layout.fields"
            );
        }
    }

    // ─── 9. Recursive-by-value struct returns error immediately ───────────────

    #[test]
    fn recursive_by_value_returns_error_not_stack_overflow() {
        let mut symbols = SymbolTable::new();
        // Reserve an ID for "Node" and create a type that refers to itself by value.
        let placeholder_id = TypeId(SymbolId(symbols.num_symbols() as u32));
        let mut sym = TypeSymbol::new(placeholder_id, "Node".into(), pub_vis());
        sym.add_field("next".into(), Type::Named("Node".into(), vec![]), pub_vis());
        symbols.define_type("Node".into(), sym);

        let r = resolver(&symbols);
        let result = r.layout_type(placeholder_id);
        assert_eq!(
            result,
            Err(LayoutError::RecursiveByValue(placeholder_id)),
            "self-referential struct must return RecursiveByValue"
        );
    }

    // ─── 10. Scale: 100-field struct resolves without error ───────────────────

    #[test]
    fn hundred_field_struct_resolves_correctly() {
        let mut symbols = SymbolTable::new();
        let tid = register_n_int_fields(&mut symbols, "Century", 100);
        let r = resolver(&symbols);
        let layout = r.layout_type(tid).unwrap();
        assert_eq!(layout.fields.len(), 100);
        assert_eq!(layout.size, 800, "100 × 8 bytes = 800");
        assert_eq!(layout.fields[0].offset, 0);
        assert_eq!(layout.fields[99].offset, 792, "last field at byte 792");
    }

    // ─── 11. Float type produces double size/align ────────────────────────────

    #[test]
    fn float_field_has_double_layout() {
        let mut symbols = SymbolTable::new();
        let placeholder_id = TypeId(SymbolId(symbols.num_symbols() as u32));
        let mut sym = TypeSymbol::new(placeholder_id, "FPoint".into(), pub_vis());
        sym.add_field("val".into(), Type::Float, pub_vis());
        let tid = symbols.define_type("FPoint".into(), sym);
        let r = resolver(&symbols);
        let layout = r.layout_type(tid).unwrap();
        assert_eq!(layout.fields[0].size, 8, "double must be 8 bytes");
        assert_eq!(layout.fields[0].align, 8, "double must be 8-aligned");
    }

    // ─── 12. Text/pointer field treated as pointer (8 bytes) ─────────────────

    #[test]
    fn text_field_has_pointer_layout() {
        let mut symbols = SymbolTable::new();
        let placeholder_id = TypeId(SymbolId(symbols.num_symbols() as u32));
        let mut sym = TypeSymbol::new(placeholder_id, "Msg".into(), pub_vis());
        sym.add_field("content".into(), Type::Text, pub_vis());
        let tid = symbols.define_type("Msg".into(), sym);
        let r = resolver(&symbols);
        let layout = r.layout_type(tid).unwrap();
        assert_eq!(layout.fields[0].size, 8, "const char* must be 8 bytes (pointer)");
    }
}
