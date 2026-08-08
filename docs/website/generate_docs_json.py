import os
import json
import re

def extract_title_and_summary(content, default_title):
    # Extract the first H1 header as the title
    title_match = re.search(r'^#\s+(.+)$', content, re.MULTILINE)
    title = title_match.group(1).strip() if title_match else default_title
    
    # Remove metadata header blocks, then find the first non-header, non-empty paragraph
    lines = content.split('\n')
    cleaned_lines = []
    in_code_block = False
    for line in lines:
        if line.strip().startswith('```'):
            in_code_block = not in_code_block
            continue
        if not in_code_block:
            # Skip empty lines, headers, blockquotes (unless introductory), and metadata tags
            if line.strip() == '' or line.strip().startswith('#') or line.strip().startswith('>'):
                continue
            cleaned_lines.append(line.strip())
            if len(cleaned_lines) > 2: # Keep the first couple of paragraphs
                break
                
    summary = " ".join(cleaned_lines)
    summary = re.sub(r'\[([^\]]+)\]\([^)]+\)', r'\1', summary) # Remove links
    summary = re.sub(r'[*_`]', '', summary) # Remove basic formatting
    if len(summary) > 150:
        summary = summary[:147] + "..."
    if not summary:
        summary = "Documentation for " + title
        
    return title, summary

def main():
    docs_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), '..'))
    website_dir = os.path.abspath(os.path.join(docs_dir, 'website'))
    
    docs_data = []
    
    # We want to traverse docs/ and find all .md files, except inside docs/website/
    for root, dirs, files in os.walk(docs_dir):
        # Skip the website directory
        if os.path.commonpath([root, website_dir]) == website_dir:
            continue
            
        for file in files:
            if file.endswith('.md'):
                file_path = os.path.join(root, file)
                rel_path = os.path.relpath(file_path, docs_dir)
                
                # Determine category based on directory structure
                category = "General"
                dir_name = os.path.dirname(rel_path)
                if not dir_name:
                    if rel_path == 'index.md':
                        category = "Welcome"
                    else:
                        category = "Getting Started"
                elif dir_name.startswith('moon'):
                    category = "Roadmap & Changelog"
                elif dir_name in ['research', 'moon/research']:
                    category = "Research"
                elif dir_name in ['reports', 'moon/reports']:
                    category = "Reports"
                elif dir_name == 'adr':
                    category = "Decision Records"
                else:
                    category = dir_name.capitalize()
                
                # Exclude build files or auto-generated ones if any
                if 'node_modules' in rel_path or '.venv' in rel_path or 'site/' in rel_path:
                    continue
                    
                with open(file_path, 'r', encoding='utf-8') as f:
                    content = f.read()
                
                # Extract clean title and summary
                default_title = os.path.splitext(file)[0].replace('_', ' ').title()
                title, summary = extract_title_and_summary(content, default_title)
                
                # Generate unique ID
                doc_id = rel_path.replace('.md', '').replace('\\', '/').replace('/', '-')
                if doc_id == 'index':
                    doc_id = 'welcome'
                    
                docs_data.append({
                    "id": doc_id,
                    "title": title,
                    "category": category,
                    "path": rel_path,
                    "content": content,
                    "summary": summary
                })
                
    # Sort docs: Welcome first, then Getting Started, then others alphabetically
    def sort_key(doc):
        cat_order = {"Welcome": 0, "Getting Started": 1, "Decision Records": 2, "Research": 3, "Reports": 4, "Roadmap & Changelog": 5}
        cat_val = cat_order.get(doc['category'], 10)
        title_val = 0 if doc['id'] == 'welcome' else 1
        return (cat_val, title_val, doc['title'])
        
    docs_data.sort(key=sort_key)
    
    # Write to Vue assets folder
    output_dir = os.path.join(website_dir, 'src', 'data')
    os.makedirs(output_dir, exist_ok=True)
    output_file = os.path.join(output_dir, 'docs.json')
    
    with open(output_file, 'w', encoding='utf-8') as f:
        json.dump(docs_data, f, indent=2, ensure_ascii=False)
        
    print(f"Generated docs.json in {output_file} with {len(docs_data)} files.")

if __name__ == '__main__':
    main()
