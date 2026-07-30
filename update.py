import os
import re

lib_path = r'c:\dev\aquila\crates\aquila_compute_aws\src\lib.rs'
with open(lib_path, 'r') as f:
    content = f.read()

helper = '''
fn format_err<E: std::error::Error>(msg: &str, e: E) -> String {
    let mut out = format!("{msg}: {e}");
    let mut src = e.source();
    while let Some(s) = src {
        out.push_str(&format!(" -> {s}"));
        src = s.source();
    }
    out.push_str(&format!(" | Debug: {e:#?}"));
    out
}
'''

content = content.replace('use uuid::Uuid;\n', 'use uuid::Uuid;\n' + helper)

def replace_fmt(match):
    s = match.group(1)
    if s == '{:?}':
        return '.map_err(|e| ComputeError::System(format_err("AWS Error", e)))'
    else:
        # e.g. "Failed to register definition: {:?}" -> "Failed to register definition", e
        s_clean = s.replace(': {:?}', '').replace(': {}', '').replace(' {}', '')
        return f'.map_err(|e| ComputeError::System(format_err("{s_clean}", e)))'

content = re.sub(r'\.map_err\(\|e\| ComputeError::System\(format!\(\"(.*?)\", e\)\)\)', replace_fmt, content)

with open(lib_path, 'w') as f:
    f.write(content)
