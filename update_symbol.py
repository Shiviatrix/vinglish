import re

with open('crates/vinglish-hir/src/symbol.rs', 'r') as f:
    content = f.read()

# Add span to VariableSymbol
content = content.replace('''    pub is_mut: bool,
    pub ty: Type,
}''', '''    pub is_mut: bool,
    pub ty: Type,
    pub span: Option<vinglish_lexer::span::Span>,
}''')

# Update add_variable signature
content = content.replace('''    pub fn add_variable(&mut self, name: String, is_mut: bool, ty: Type) -> VariableId {
        let index = self.symbols.len();
        let id = VariableId(index);
        self.symbols.push(SymbolKind::Variable(VariableSymbol {
            id,
            name: name.clone(),
            is_mut,
            ty,
        }));
        id
    }''', '''    pub fn add_variable(
        &mut self, 
        name: String, 
        is_mut: bool, 
        ty: Type, 
        span: Option<vinglish_lexer::span::Span>
    ) -> VariableId {
        let index = self.symbols.len();
        let id = VariableId(index);
        self.symbols.push(SymbolKind::Variable(VariableSymbol {
            id,
            name: name.clone(),
            is_mut,
            ty,
            span,
        }));
        id
    }''')

with open('crates/vinglish-hir/src/symbol.rs', 'w') as f:
    f.write(content)
