#!/usr/bin/env python3
"""Extract #[cfg(test)] modules from source files, update mod.rs declarations."""

import os

BASE = "/home/casibbald/Workspace/microscaler/may_redis/src"

# (relative_path, test_filename)
files_to_extract = [
    ("codec/roundtrip.rs", "roundtrip_tests.rs"),
    ("core/from_value.rs", "from_value_tests.rs"),
    ("codec/reader.rs", "reader_tests.rs"),
    ("client/in_memory.rs", "in_memory_tests.rs"),
    ("protocol/builder.rs", "builder_tests.rs"),
]

for rel, test_name in files_to_extract:
    src = os.path.join(BASE, rel)
    test_file = os.path.join(BASE, rel.rsplit("/", 1)[0] if "/" in rel else "", test_name)
    # handle flat module: protocol/builder.rs -> protocol/builder_tests.rs
    if "/" in rel:
        dir_part, base = rel.rsplit("/", 1)
        test_file = os.path.join(BASE, dir_part, test_name)
    else:
        test_file = os.path.join(BASE, test_name)
    
    with open(src, "r") as f:
        content = f.read()
    
    lines = content.split("\n")
    
    # Find #[cfg(test)]
    split_line = -1
    for i, line in enumerate(lines):
        if line.strip() == "#[cfg(test)]":
            split_line = i
            break
    
    if split_line == -1:
        print(f"SKIP {rel}: no #[cfg(test)] found")
        continue
    
    prod_lines = lines[:split_line]
    # Remove trailing blank lines from production
    while prod_lines and prod_lines[-1].strip() == "":
        prod_lines.pop()
    # Add final newline
    prod_content = "\n".join(prod_lines) + "\n"
    
    test_content = "\n".join(lines[split_line:])
    
    with open(test_file, "w") as f:
        f.write(test_content)
    
    with open(src, "w") as f:
        f.write(prod_content)
    
    prod_count = len(prod_lines)
    test_count = len(lines[split_line:])
    total = len(lines)
    print(f"EXTRACTED {rel}: {prod_count} prod + {test_count} test = {total} total -> {test_name}")

print()

# Now update mod.rs declarations
mod_updates = {
    "codec/mod.rs": "    pub mod roundtrip_tests;\n",
    "core/mod.rs": "pub mod from_value_tests;\n",
    "codec/reader.rs": "    mod reader_tests;\n",
    "client/in_memory.rs": "    mod in_memory_tests;\n",
    "protocol/builder.rs": "    mod builder_tests;\n",
}

for mod_rel, declaration in mod_updates.items():
    mod_path = os.path.join(BASE, mod_rel)
    with open(mod_path, "r") as f:
        content = f.read()
    
    if declaration.strip() in content:
        print(f"SKIP {mod_rel}: '{declaration.strip()}' already present")
        continue
    
    if "    pub mod roundtrip_tests" in declaration:
        # codec/mod.rs: find the #[cfg(test)] block with roundtrip, add after
        marker = "pub mod roundtrip;"
        content = content.replace(
            marker,
            marker + "\n" + declaration.strip()
        )
    elif "pub mod from_value_tests" in declaration:
        # core/mod.rs: add before the re-export block, after value module
        marker = "pub mod value;"
        content = content.replace(
            marker,
            marker + "\n\n" + declaration.strip()
        )
    else:
        # reader.rs, in_memory.rs, builder.rs: add inside the existing #[cfg(test)] block
        # Find the mod name, add the test module inside
        if "reader_tests" in declaration:
            marker = "#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n#[cfg(test)]\nmod tests {"
        elif "in_memory_tests" in declaration:
            marker = "#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n#[cfg(test)]\nmod tests {"
        elif "builder_tests" in declaration:
            marker = "#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n#[cfg(test)]\nmod tests {"
        else:
            marker = None
        
        if marker:
            content = content.replace(marker, marker + "\n    " + declaration.strip())
            print(f"UPDATED {mod_rel}: added '{declaration.strip()}' inside #[cfg(test)] block")
        else:
            print(f"SKIP {mod_rel}: couldn't find insertion point")
    
    with open(mod_path, "w") as f:
        f.write(content)

print("\nDone!")
