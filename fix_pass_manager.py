import re
import os

def update_lib():
    with open('crates/vinglish-opt/src/lib.rs', 'r') as f:
        content = f.read()

    content = content.replace(
        'fn run(&mut self, module: &mut MirModule<V>) -> PassStats;',
        'fn run(&mut self, module: &mut MirModule<V>, symbol_table: &vinglish_hir::symbol::SymbolTable) -> PassStats;'
    )

    content = content.replace(
        'let stats = pass.run(module);',
        'let stats = pass.run(module, symbol_table);'
    )
    with open('crates/vinglish-opt/src/lib.rs', 'w') as f:
        f.write(content)

def update_passes():
    for file in os.listdir('crates/vinglish-opt/src'):
        if file.endswith('.rs') and file != 'lib.rs':
            with open('crates/vinglish-opt/src/' + file, 'r') as f:
                content = f.read()
            if 'fn run(&mut self, module: &mut MirModule<V>) -> PassStats' in content:
                content = content.replace(
                    'fn run(&mut self, module: &mut MirModule<V>) -> PassStats',
                    'fn run(&mut self, module: &mut MirModule<V>, _symbol_table: &vinglish_hir::symbol::SymbolTable) -> PassStats'
                )
            elif 'fn run(&mut self, module: &mut MirModule<SsaValueId>) -> PassStats' in content:
                content = content.replace(
                    'fn run(&mut self, module: &mut MirModule<SsaValueId>) -> PassStats',
                    'fn run(&mut self, module: &mut MirModule<SsaValueId>, _symbol_table: &vinglish_hir::symbol::SymbolTable) -> PassStats'
                )
            with open('crates/vinglish-opt/src/' + file, 'w') as f:
                f.write(content)

update_lib()
update_passes()
