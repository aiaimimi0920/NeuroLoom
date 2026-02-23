import os
import re

examples_dir = 'examples'

for root, _, files in os.walk(examples_dir):
    for file in files:
        if file == 'main.rs':
            filepath = os.path.join(root, file)
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()

            # Determine the auth method based on the preset name (folder name usually, or looking at the code)
            auth_method = "with_api_key"
            if "iflow" in filepath:
                auth_method = "with_cookie"
            elif "vertex" in filepath:
                auth_method = "with_service_account_json"
            elif "anthropic" in filepath:
                auth_method = "with_anthropic_api_key"

            # Check if an API key is parsed
            if 'dummy_credential' in content:
                # Fix the _api_key naming Issue (remove the underscore)
                content = re.sub(r'let\s+(_?api_key)\s*=', r'let api_key =', content)
                content = re.sub(r'let\s+mut\s+(_?api_key)\s*=', r'let mut api_key =', content)

                # Check if the auth string is missing
                if auth_method not in content:
                    # We need to insert it
                    # Find: .expect("Preset should exist")
                    # Replace with: .expect("Preset should exist")\n        .with_api_key(api_key)
                    
                    replacement = f'.expect("Preset should exist")\n        .{auth_method}(api_key)'
                    
                    if '.expect("Preset should exist")' in content:
                        content = content.replace('.expect("Preset should exist")', replacement, 1)
                    else:
                        print(f"Match not found in {filepath} for injection!")

            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"Processed {filepath}")

print("Done fixing examples.")
