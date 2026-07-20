import re

with open('crates/vinglish-hir/src/symbol.rs', 'r') as f:
    content = f.read()

trait_code = """
pub trait HasSymbolId {
    fn symbol_id(&self) -> SymbolId;
}

impl HasSymbolId for VariableId {
    fn symbol_id(&self) -> SymbolId { self.0 }
}

impl HasSymbolId for SsaValueId {
    fn symbol_id(&self) -> SymbolId { self.0.0 }
}
"""

if "pub trait HasSymbolId" not in content:
    content += trait_code

with open('crates/vinglish-hir/src/symbol.rs', 'w') as f:
    f.write(content)

with open('crates/vinglish-opt/src/lib.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub trait OptimizationPass<V: Clone + Copy + Display + Eq + Hash> {',
    'pub trait OptimizationPass<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> {'
)
content = content.replace(
    'pub struct PassManager<V: Clone + Copy + Display + Eq + Hash> {',
    'pub struct PassManager<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> {'
)
content = content.replace(
    'impl<V: Clone + Copy + Display + Eq + Hash> Default for PassManager<V> {',
    'impl<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> Default for PassManager<V> {'
)
content = content.replace(
    'impl<V: Clone + Copy + Display + Eq + Hash> PassManager<V> {',
    'impl<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> PassManager<V> {'
)

with open('crates/vinglish-opt/src/lib.rs', 'w') as f:
    f.write(content)

import os
for file in os.listdir('crates/vinglish-opt/src'):
    if file.endswith('.rs') and file != 'lib.rs':
        with open('crates/vinglish-opt/src/' + file, 'r') as f:
            content = f.read()
        content = content.replace(
            'impl<V: Clone + Copy + Display + Eq + Hash> OptimizationPass<V>',
            'impl<V: Clone + Copy + Display + Eq + Hash + vinglish_hir::symbol::HasSymbolId> OptimizationPass<V>'
        )
        with open('crates/vinglish-opt/src/' + file, 'w') as f:
            f.write(content)

