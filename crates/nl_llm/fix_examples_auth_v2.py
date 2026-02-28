import os
import re

examples_dir = 'examples'

for root, _, files in os.walk(examples_dir):
    for file in files:
        if file == 'main.rs':
            filepath = os.path.join(root, file)
            with open(filepath, 'r', encoding='utf-8') as f:
                content = f.read()

            if 'dummy_credential' in content:
                # 1. Standardize variable name to `api_key` instead of `_api_key`
                content = re.sub(r'let\s+(_?api_key)\s*=', r'let api_key =', content)
                content = re.sub(r'let\s+mut\s+(_?api_key)\s*=', r'let mut api_key =', content)

                # 2. Determine correct auth method
                auth_method = "with_api_key"
                if "iflow" in filepath:
                    auth_method = "with_cookie"
                elif "vertex" in filepath:
                    auth_method = "with_service_account_json"
                elif "anthropic" in filepath:
                    auth_method = "with_anthropic_api_key"
                elif "antigravity" in filepath or "gemini_cli" in filepath:
                    auth_method = "with_gemini_cli_oauth"

                # 3. Strip all existing method calls
                content = re.sub(r'\s*\.with_api_key\([^\)]*\)', '', content)
                content = re.sub(r'\s*\.with_cookie\([^\)]*\)', '', content)
                content = re.sub(r'\s*\.with_service_account_json\([^\)]*\)', '', content)
                content = re.sub(r'\s*\.with_anthropic_api_key\([^\)]*\)', '', content)
                content = re.sub(r'\s*\.with_gemini_cli_oauth\([^\)]*\)', '', content)

                # 4. Inject the correct method
                replacement = f'.expect("Preset should exist")\n        .{auth_method}(api_key)'
                if '.expect("Preset should exist")' in content:
                    content = content.replace('.expect("Preset should exist")', replacement, 1)

            with open(filepath, 'w', encoding='utf-8') as f:
                f.write(content)
            print(f"Processed {filepath}")

print("Done fixing examples.")
