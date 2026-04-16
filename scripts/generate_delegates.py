import re

import sys

with open('libs/aiome-core-contracts/src/traits.rs', 'r') as f:
    traits_content = f.read()

target_traits = [
    "AgentEvolver", "AuditStore", "BiomeRegistry", "ChatStore",
    "FederationRegistry", "HarnessRegistryOps", "ImmuneSystemOps",
    "KarmaRegistry", "SoulStore", "SystemStateOps", "TaskRegistry",
    "JobQueue", "EvaluationOps"
]

output = ""
for trait in target_traits:
    pattern = r'pub trait ' + trait + r'(?:[^\{]*)\{([\s\S]*?)\n\}'
    match = re.search(pattern, traits_content)
    if not match:
        continue
    
    methods_block = match.group(1)
    # Extract method signatures
    method_pattern = r'(async\s+fn\s+([a-zA-Z0-9_]+)\s*\((.*?)\)(?:\s*->\s*(.*?))?;)'
    
    output += f"\n#[async_trait]\nimpl {trait} for RealJobQueue {{\n"
    for m in re.finditer(method_pattern, methods_block):
        full_sig = m.group(1)
        name = m.group(2)
        args_str = m.group(3)
        ret = m.group(4) or "()"
        
        # Clean args to get variable names
        args = args_str.split(',')
        var_names = []
        for arg in args:
            arg = arg.strip()
            if not arg or arg == '&self': continue
            var_name = arg.split(':')[0].strip()
            # remove mut
            var_name = var_name.replace('mut ', '')
            var_names.append(var_name)
            
        call_args = ", ".join(var_names)
        
        output += f"    {full_sig.replace(';', ' {')}\n"
        output += f"        self.inner.{name}({call_args}).await\n"
        output += f"    }}\n"
    output += "}\n"

with open('/tmp/delegates.rs', 'w') as f:
    f.write(output)
print("Generated delegates to /tmp/delegates.rs")
