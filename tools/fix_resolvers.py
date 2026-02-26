import os
import re

directory = r"C:\Users\Public\nas_home\AI\GameEditor\NeuroLoom\crates\nl_llm_v2\src\model"

replacement_code = """    fn context_window_hint(&self, model: &str) -> (usize, usize) {
        self.inner.context_window_hint(model)
    }

    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, crate::model::resolver::Modality)> {
        self.inner.intelligence_and_modality(model)
    }"""

for filename in os.listdir(directory):
    if filename.endswith(".rs") and filename not in ["router.rs", "resolver.rs", "default.rs", "mod.rs", "spark.rs", "openai.rs"]:
        filepath = os.path.join(directory, filename)
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()
            
        if "fn intelligence_and_modality" not in content and "impl ModelResolver for" in content:
            # Replaces the context_window_hint to gracefully insert intelligence_and_modality
            # Let's make it robust by looking for standard 'context_window_hint'
            
            # Using regex to replace the last implemented method in standard resolvers.
            pattern = re.compile(r"    fn context_window_hint\(\&self, model\: \&str\) \-\> \(usize\, usize\) \{(.*?)\}", re.DOTALL)
            match = pattern.search(content)
            if match:
                original_hint = match.group(0)
                new_str = f"{original_hint}\n\n    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, crate::model::resolver::Modality)> {{\n        self.inner.intelligence_and_modality(model)\n    }}"
                new_content = content.replace(original_hint, new_str)
                with open(filepath, "w", encoding="utf-8") as f:
                    f.write(new_content)
                print(f"Updated {filename}")
            else:
                print(f"Pattern not found in {filename}")

# Note: Some resolvers might have slightly different names or logic inside context_window_hint, 
# so I'll just append it before the last `}` of `impl ModelResolver for X {}`

def robust_update():
    for filename in os.listdir(directory):
        if filename.endswith(".rs") and filename not in ["router.rs", "resolver.rs", "default.rs", "mod.rs"]:
            filepath = os.path.join(directory, filename)
            with open(filepath, "r", encoding="utf-8") as f:
                content = f.read()

            if "fn intelligence_and_modality" not in content and "impl ModelResolver for" in content:
                # Find "impl ModelResolver for" block
                start_match = re.search(r"impl ModelResolver for [^{]+\{", content)
                if start_match:
                    start_idx = start_match.end()
                    # Find matching closing brace
                    depth = 1
                    end_idx = -1
                    for i in range(start_idx, len(content)):
                        if content[i] == '{':
                            depth += 1
                        elif content[i] == '}':
                            depth -= 1
                            if depth == 0:
                                end_idx = i
                                break
                    if end_idx != -1:
                        # inject before end_idx
                        injection = "\n    fn intelligence_and_modality(&self, model: &str) -> Option<(f32, crate::model::resolver::Modality)> {\n        self.inner.intelligence_and_modality(model)\n    }\n"
                        new_content = content[:end_idx] + injection + content[end_idx:]
                        with open(filepath, "w", encoding="utf-8") as f:
                            f.write(new_content)
                        print(f"Successfully robust-updated {filename}")

robust_update()
