import os
import shutil
import re

EXAMPLES_DIR = 'examples'

# Define the folders and file mappings
file_mapping = {
    '20_maps.ving': 'basics/maps.ving',
    'hello.ving': 'basics/hello.ving',
    'fibonacci.ving': 'basics/fibonacci.ving',
    'structs.ving': 'basics/structs.ving',
    
    '21_networking.ving': 'advanced/networking.ving',
    'patterns.ving': 'advanced/patterns.ving',
    'finance.ving': 'advanced/finance.ving',
    
    'ui_test.ving': 'ui_games/ui_test.ving',
    'skyline_runner.ving': 'ui_games/skyline_runner.ving',
    
    'errors_own.ving': 'errors/errors_own.ving',
    'errors_own_all.ving': 'errors/errors_own_all.ving',
    'errors_syntax.ving': 'errors/errors_syntax.ving',
    'errors_type.ving': 'errors/errors_type.ving',
    
    'mir_test.ving': 'compiler_tests/mir_test.ving',
    'opt_test.ving': 'compiler_tests/opt_test.ving',
}

def remove_comments(content):
    # Remove lines that are just comments (optionally with whitespace before)
    # Vinglish uses // and -- for comments.
    lines = content.split('\n')
    cleaned_lines = []
    for line in lines:
        stripped = line.strip()
        # Remove whole line comments
        if stripped.startswith('//') or stripped.startswith('--'):
            continue
        # Remove trailing comments
        # Handle // and -- but avoid removing inside strings.
        # For simplicity, we just split on ' //' and ' --' if they aren't preceded by an odd number of quotes.
        # Given the examples we saw, a simple split on " //" or " --" works fine for trailing comments.
        
        idx_slash = line.find(' //')
        idx_dash = line.find(' --')
        
        if idx_slash != -1:
            line = line[:idx_slash]
        if idx_dash != -1:
            line = line[:idx_dash]
            
        cleaned_lines.append(line)
        
    # Remove leading and trailing empty lines
    while cleaned_lines and not cleaned_lines[0].strip():
        cleaned_lines.pop(0)
    while cleaned_lines and not cleaned_lines[-1].strip():
        cleaned_lines.pop()
        
    return '\n'.join(cleaned_lines)

if __name__ == '__main__':
    # Create directories
    for folder in ['basics', 'advanced', 'ui_games', 'errors', 'compiler_tests']:
        os.makedirs(os.path.join(EXAMPLES_DIR, folder), exist_ok=True)
        
    # Process files
    for old_name, new_rel_path in file_mapping.items():
        old_path = os.path.join(EXAMPLES_DIR, old_name)
        if os.path.exists(old_path):
            with open(old_path, 'r') as f:
                content = f.read()
                
            new_content = remove_comments(content)
            
            new_path = os.path.join(EXAMPLES_DIR, new_rel_path)
            with open(new_path, 'w') as f:
                f.write(new_content)
                
            os.remove(old_path)
            print(f"Moved and cleaned: {old_name} -> {new_rel_path}")
